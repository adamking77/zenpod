//! An episode's loudness shape: 360 points, decoded once from the downloaded file.
use std::path::Path;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

pub const POINTS: usize = 360;

pub fn compute(path: &Path) -> Result<Vec<f32>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .map_err(|e| e.to_string())?;
    let track = format.default_track(TrackType::Audio).ok_or("no audio track")?;
    let track_id = track.id;
    let params = track.codec_params.as_ref().and_then(|p| p.audio()).ok_or("no codec parameters")?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| e.to_string())?;

    // RMS per decoded packet; bucketed below. An hour is ~150k packets, a few hundred KB.
    let (mut rms, mut buf) = (Vec::<f32>::new(), Vec::<f32>::new());
    loop {
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(Error::IoError(_)) => break,
            Err(e) => return Err(e.to_string()),
        };
        if packet.track_id != track_id {
            continue;
        }
        let Ok(audio) = decoder.decode(&packet) else { continue };
        buf.resize(audio.samples_interleaved(), 0.0);
        audio.copy_to_slice_interleaved(&mut buf);
        if !buf.is_empty() {
            rms.push((buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt());
        }
    }
    Ok(shape(&rms))
}

/// Mean loudness per bucket, scaled so the loudest is 1, never below a quiet floor.
pub fn shape(rms: &[f32]) -> Vec<f32> {
    if rms.is_empty() {
        return vec![0.04; POINTS];
    }
    let b: Vec<f32> = (0..POINTS)
        .map(|i| {
            let (a, z) = (i * rms.len() / POINTS, ((i + 1) * rms.len() / POINTS).max(i * rms.len() / POINTS + 1).min(rms.len()));
            rms[a.min(rms.len() - 1)..z].iter().sum::<f32>() / (z - a.min(rms.len() - 1)).max(1) as f32
        })
        .collect();
    let hi = b.iter().cloned().fold(0.0, f32::max).max(1e-6);
    b.iter().map(|v| (v / hi).max(0.04)).collect()
}

pub fn to_blob(p: &[f32]) -> Vec<u8> {
    p.iter().flat_map(|v| v.to_le_bytes()).collect()
}

pub fn from_blob(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes() {
        let s = shape(&(0..1000).map(|i| i as f32).collect::<Vec<_>>());
        assert_eq!(s.len(), POINTS);
        assert!((s[POINTS - 1] - 1.0).abs() < 1e-6 && s[0] <= s[POINTS / 2]);
        assert_eq!(shape(&[0.5; 10]).len(), POINTS); // fewer packets than points
        assert_eq!(from_blob(&to_blob(&s)), s);
    }

    #[test]
    fn decodes_a_real_file() {
        // Uses any episode already downloaded by the app; skipped on a clean machine.
        let dir = dirs_audio();
        let Some(f) = std::fs::read_dir(&dir).ok().and_then(|mut d| d.find_map(|e| e.ok().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "mp3" || x == "m4a")))) else { return };
        let t = std::time::Instant::now();
        let p = compute(&f).unwrap();
        eprintln!("peaks for {} in {:?}", f.display(), t.elapsed());
        assert_eq!(p.len(), POINTS);
        assert!(p.iter().any(|v| *v > 0.5));
    }

    fn dirs_audio() -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("Library/Application Support/me.adamking.listener/audio")
    }
}
