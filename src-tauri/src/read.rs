//! Read to me: text becomes an episode. One reading at a time, in the order asked for, in pieces small enough that
//! progress moves steadily and a failure costs one piece. Finished pieces are kept until the episode is made, so
//! stopping, quitting or an error never re-sends what's already been read.
//!
//! Text arrives as plain paragraphs separated by blank lines; a line starting "# " is a heading, which becomes a
//! chapter.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{store, voice, Core};

static WAKE: LazyLock<tokio::sync::Notify> = LazyLock::new(tokio::sync::Notify::new);

/// Silence between pieces, so paragraphs breathe where the pieces join.
const GAP: f64 = 0.35;

pub fn dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("app data").join("read")
}

fn changed(app: &AppHandle) {
    let _ = app.emit("reads", ());
}

/// Something new is queued: the reader picks it up.
pub fn wake() {
    WAKE.notify_one();
}

/// The reader: runs for the life of the app, one reading at a time.
pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let next = store::next_queued(&app.state::<Core>().db.lock().unwrap()).ok().flatten();
            match next {
                Some(r) => {
                    if let Err(e) = run(&app, &r).await {
                        let db = app.state::<Core>();
                        let db = db.db.lock().unwrap();
                        // Stopped while reading: it's back to waiting, not failed.
                        if store::read(&db, r.id).ok().flatten().is_some_and(|x| x.state == "reading") {
                            let _ = store::set_read_state(&db, r.id, "failed", Some(&e));
                        }
                    }
                    changed(&app);
                }
                None => WAKE.notified().await,
            }
        }
    });
}

fn state_of(app: &AppHandle, id: i64) -> Option<String> {
    store::read(&app.state::<Core>().db.lock().unwrap(), id).ok().flatten().map(|r| r.state)
}

async fn run(app: &AppHandle, r: &store::ReadRow) -> Result<(), String> {
    let (voice, text) = {
        let core = app.state::<Core>();
        let db = core.db.lock().unwrap();
        store::set_read_state(&db, r.id, "reading", None).map_err(|e| e.to_string())?;
        let show_voice = store::show_voice(&db, r.show_id);
        (voice::resolve(&db, show_voice.as_deref()), std::fs::read_to_string(dir(app).join(format!("{}.txt", r.id))))
    };
    changed(app);
    let text = text.map_err(|_| "The text for this is gone. Add it again.".to_string())?;
    let pieces = split(&text, voice.cap());
    if pieces.is_empty() {
        return Err("There's nothing to read in this.".into());
    }
    let work = dir(app).join(r.id.to_string());
    // A different number of pieces means a different voice cut the text differently: start the pieces again.
    if r.pieces as usize != pieces.len() {
        let _ = std::fs::remove_dir_all(&work);
    }
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;

    let mut files = Vec::with_capacity(pieces.len());
    for (i, p) in pieces.iter().enumerate() {
        if let Some(f) = piece_file(&work, i) {
            files.push(f);
            continue;
        }
        if state_of(app, r.id).as_deref() != Some("reading") {
            return Ok(()); // stopped
        }
        progress(app, r.id, i, pieces.len());
        let audio = match voice::speak(&voice, &p.text).await {
            Ok(a) => a,
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                voice::speak(&voice, &p.text).await?
            }
        };
        let f = work.join(format!("{i}.{}", audio.ext));
        std::fs::write(&f, &audio.bytes).map_err(|e| e.to_string())?;
        files.push(f);
    }
    progress(app, r.id, pieces.len(), pieces.len());
    if state_of(app, r.id).as_deref() != Some("reading") {
        return Ok(());
    }

    // Join: decode, one rate, mono, 350 ms between pieces; then AAC.
    let wav = work.join("joined.wav");
    let out = dir(app).join(format!("{}.m4a", r.id));
    let lengths = {
        let (files, wav) = (files.clone(), wav.clone());
        tauri::async_runtime::spawn_blocking(move || join(&files, &wav)).await.map_err(|e| e.to_string())??
    };
    let conv = tokio::process::Command::new("afconvert")
        .args(["-f", "m4af", "-d", "aac", "-q", "127", "-s", "3"])
        .arg(&wav)
        .arg(&out)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !conv.status.success() {
        return Err(format!("The audio couldn't be saved: {}", String::from_utf8_lossy(&conv.stderr).trim()));
    }

    let (vtt, chapters, total) = timings(&pieces, &lengths);
    let core = app.state::<Core>();
    let db = core.db.lock().unwrap();
    if store::read(&db, r.id).ok().flatten().is_none_or(|x| x.state != "reading") {
        let _ = std::fs::remove_file(&out);
        return Ok(());
    }
    let first = text.split("\n\n").map(str::trim).find(|b| !b.is_empty() && !b.starts_with("# ")).unwrap_or("");
    let description = format!("<p>From {}</p><p>{}</p>", source_html(&r.source), esc(&first.chars().take(600).collect::<String>()));
    let episode = store::finish_read(&db, &store::Made {
        show: r.show_id,
        read: r.id,
        title: &r.title,
        duration: total,
        path: &out.to_string_lossy(),
        description: &description,
        transcript: "zenpod:read",
        chapters: chapters.is_some().then_some("zenpod:read"),
    })
    .map_err(|e| e.to_string())?;
    drop(db);
    let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    for (kind, body) in [("transcripts", Some(vtt)), ("chapters", chapters)] {
        let d = data.join(kind);
        let _ = std::fs::create_dir_all(&d);
        let f = d.join(format!("{episode}.txt"));
        match body {
            Some(b) => {
                let _ = std::fs::write(&f, b);
            }
            None => {
                let _ = std::fs::remove_file(&f);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&work);
    let _ = std::fs::remove_file(dir(app).join(format!("{}.txt", r.id)));
    let _ = app.emit("library", ());
    let _ = app.emit("episode", episode);
    Ok(())
}

fn progress(app: &AppHandle, id: i64, done: usize, pieces: usize) {
    let _ = store::set_read_progress(&app.state::<Core>().db.lock().unwrap(), id, done as i64, pieces as i64);
    changed(app);
}

fn piece_file(work: &Path, i: usize) -> Option<PathBuf> {
    ["mp3", "wav"].iter().map(|e| work.join(format!("{i}.{e}"))).find(|p| p.exists())
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn source_html(src: &str) -> String {
    if src.starts_with("http://") || src.starts_with("https://") {
        format!("<a href=\"{0}\">{0}</a>", esc(src))
    } else {
        esc(src)
    }
}

// ---------- text into pieces ----------

#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub text: String,
    /// The heading this piece starts with: a chapter begins here.
    pub heading: Option<String>,
}

/// Headings first, then paragraphs, then sentences; a piece never cuts a sentence (unless one sentence is longer than
/// a whole piece, when it's cut between words).
pub fn split(text: &str, cap: usize) -> Vec<Piece> {
    let mut out: Vec<Piece> = vec![];
    let mut cur = Piece { text: String::new(), heading: None };
    let flush = |cur: &mut Piece, out: &mut Vec<Piece>| {
        if !cur.text.trim().is_empty() {
            out.push(Piece { text: cur.text.trim().to_string(), heading: cur.heading.take() });
        }
        *cur = Piece { text: String::new(), heading: None };
    };
    // Paragraphs join with a blank line, sentences of one paragraph with a space.
    let push = |cur: &mut Piece, out: &mut Vec<Piece>, chunk: &str, sep: &str| {
        if !cur.text.is_empty() && cur.text.chars().count() + chunk.chars().count() + sep.len() > cap {
            flush(cur, out);
        }
        if !cur.text.is_empty() {
            cur.text.push_str(sep);
        }
        cur.text.push_str(chunk);
    };
    for block in text.split("\n\n").map(|b| b.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|b| !b.is_empty()) {
        if let Some(h) = block.strip_prefix("# ") {
            flush(&mut cur, &mut out);
            let h = h.trim().to_string();
            cur.heading = Some(h.clone());
            cur.text = if h.ends_with(['.', '!', '?', ':']) { h } else { format!("{h}.") };
            continue;
        }
        if block.chars().count() <= cap {
            push(&mut cur, &mut out, &block, "\n\n");
            continue;
        }
        let mut sep = "\n\n";
        for s in sentences(&block) {
            if s.chars().count() <= cap {
                push(&mut cur, &mut out, &s, sep);
                sep = " ";
                continue;
            }
            let mut part = String::new();
            for w in s.split(' ') {
                if !part.is_empty() && part.chars().count() + w.chars().count() + 1 > cap {
                    push(&mut cur, &mut out, &part, sep);
                    sep = " ";
                    part.clear();
                }
                if !part.is_empty() {
                    part.push(' ');
                }
                part.push_str(w);
            }
            if !part.is_empty() {
                push(&mut cur, &mut out, &part, sep);
                sep = " ";
            }
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// Sentences end at . ! ? or … followed by a space and a capital, a digit or an opening quote.
pub fn sentences(text: &str) -> Vec<String> {
    let c: Vec<char> = text.chars().collect();
    let (mut out, mut start) = (vec![], 0);
    let mut i = 0;
    while i < c.len() {
        if matches!(c[i], '.' | '!' | '?' | '…') {
            let mut j = i + 1;
            while j < c.len() && matches!(c[j], '"' | '\'' | '”' | '’' | ')' | '.') {
                j += 1;
            }
            if j < c.len() && c[j] == ' ' && c.get(j + 1).is_some_and(|n| n.is_uppercase() || n.is_ascii_digit() || matches!(n, '"' | '“' | '\'' | '‘' | '(')) {
                out.push(c[start..j].iter().collect::<String>().trim().to_string());
                start = j + 1;
                i = j;
            }
        }
        i += 1;
    }
    let rest: String = c[start.min(c.len())..].iter().collect::<String>().trim().to_string();
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

/// A transcript that follows the voice, chapters at headings, and the total length. Each piece's decoded length is
/// shared among its sentences by how many characters each has.
pub fn timings(pieces: &[Piece], lengths: &[f64]) -> (String, Option<String>, f64) {
    let mut vtt = String::from("WEBVTT\n\n");
    let mut chapters = vec![];
    let mut t = 0.0;
    for (n, (p, len)) in pieces.iter().zip(lengths).enumerate() {
        if n > 0 {
            t += GAP;
        }
        if let Some(h) = &p.heading {
            chapters.push(serde_json::json!({ "title": h, "startTime": (t * 1000.0).round() / 1000.0 }));
        }
        let ss: Vec<String> = p.text.split("\n\n").flat_map(sentences).collect();
        let total: usize = ss.iter().map(|s| s.chars().count().max(1)).sum::<usize>().max(1);
        let mut at = t;
        for s in &ss {
            let d = len * s.chars().count().max(1) as f64 / total as f64;
            vtt.push_str(&format!("{} --> {}\n{}\n\n", stamp(at), stamp(at + d), s));
            at += d;
        }
        t += len;
    }
    let chapters = (!chapters.is_empty()).then(|| serde_json::json!({ "version": "1.2.0", "chapters": chapters }).to_string());
    (vtt, chapters, t)
}

fn stamp(s: f64) -> String {
    let ms = (s * 1000.0).round() as u64;
    format!("{:02}:{:02}:{:02}.{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
}

// ---------- joining the audio ----------

/// Decode each piece, mix to mono at the first piece's rate, and write one 16-bit WAV with a gap between pieces.
/// Returns each piece's length in seconds.
fn join(files: &[PathBuf], out: &Path) -> Result<Vec<f64>, String> {
    use std::io::{Seek, SeekFrom, Write};
    let mut w = std::io::BufWriter::new(std::fs::File::create(out).map_err(|e| e.to_string())?);
    w.write_all(&voice::wav_header(0, 0)).map_err(|e| e.to_string())?;
    let (mut rate, mut written, mut lengths) = (0u32, 0u64, vec![]);
    for (n, f) in files.iter().enumerate() {
        let (pcm, r) = decode_mono(f).map_err(|e| format!("A piece couldn't be read back: {e}"))?;
        if rate == 0 {
            rate = r;
        }
        let pcm = if r == rate { pcm } else { resample(&pcm, r, rate) };
        if n > 0 {
            let gap = (GAP * rate as f64) as u64;
            for _ in 0..gap {
                w.write_all(&0i16.to_le_bytes()).map_err(|e| e.to_string())?;
            }
            written += gap;
        }
        for s in &pcm {
            w.write_all(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).map_err(|e| e.to_string())?;
        }
        written += pcm.len() as u64;
        lengths.push(pcm.len() as f64 / rate as f64);
    }
    let mut f = w.into_inner().map_err(|e| e.to_string())?;
    f.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    f.write_all(&voice::wav_header(rate.max(1), (written * 2) as u32)).map_err(|e| e.to_string())?;
    Ok(lengths)
}

fn decode_mono(path: &Path) -> Result<(Vec<f32>, u32), String> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::errors::Error;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

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
    let mut decoder = symphonia::default::get_codecs().make_audio_decoder(params, &AudioDecoderOptions::default()).map_err(|e| e.to_string())?;
    let (mut out, mut buf, mut rate) = (Vec::<f32>::new(), Vec::<f32>::new(), params.sample_rate.unwrap_or(0));
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
        let ch = audio.spec().channels().count().max(1);
        rate = audio.spec().rate();
        buf.resize(audio.samples_interleaved(), 0.0);
        audio.copy_to_slice_interleaved(&mut buf);
        out.extend(buf.chunks(ch).map(|f| f.iter().sum::<f32>() / ch as f32));
    }
    if rate == 0 {
        return Err("no sample rate".into());
    }
    Ok((out, rate))
}

/// Linear: plenty for speech joined from one voice that changed rate between pieces.
fn resample(pcm: &[f32], from: u32, to: u32) -> Vec<f32> {
    if pcm.is_empty() {
        return vec![];
    }
    let n = (pcm.len() as f64 * to as f64 / from as f64) as usize;
    (0..n)
        .map(|i| {
            let x = i as f64 * from as f64 / to as f64;
            let (a, f) = (x.floor() as usize, (x - x.floor()) as f32);
            let b = (a + 1).min(pcm.len() - 1);
            pcm[a.min(pcm.len() - 1)] * (1.0 - f) + pcm[b] * f
        })
        .collect()
}

// ---------- text from files ----------

#[derive(Serialize)]
pub struct Doc {
    pub title: String,
    pub text: String,
}

/// PDF, Markdown, text, Word, RTF or HTML on disk, as plain paragraphs with "# " headings.
pub fn read_file(path: &Path) -> Result<Doc, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").replace(['-', '_'], " ");
    let (text, title) = match ext.as_str() {
        "md" | "markdown" => {
            let raw = std::fs::read_to_string(path).map_err(|_| "That file couldn't be opened.")?;
            let t = markdown(&raw);
            let title = t.lines().find_map(|l| l.strip_prefix("# ")).map(str::to_string);
            (t, title)
        }
        "txt" | "text" => (paragraphs(&std::fs::read_to_string(path).map_err(|_| "That file couldn't be opened.")?), None),
        "pdf" => {
            let out = std::process::Command::new("osascript")
                .args(["-l", "JavaScript", "-e", PDF_TEXT])
                .arg(path)
                .output()
                .map_err(|e| e.to_string())?;
            let t = String::from_utf8_lossy(&out.stdout).to_string();
            if t.trim().len() < 40 {
                return Err("This PDF has no text in it, only images of pages.".into());
            }
            (pdf_paragraphs(&t), None)
        }
        "docx" | "doc" | "rtf" | "rtfd" | "html" | "htm" | "odt" => {
            let out = std::process::Command::new("textutil")
                .args(["-convert", "txt", "-stdout"])
                .arg(path)
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err("That file couldn't be opened.".into());
            }
            (paragraphs(&String::from_utf8_lossy(&out.stdout)), None)
        }
        _ => return Err("Zenpod can read PDF, Markdown, text, Word, RTF and HTML files.".into()),
    };
    if text.trim().is_empty() {
        return Err("There's nothing to read in that file.".into());
    }
    Ok(Doc { title: title.unwrap_or(stem), text })
}

const PDF_TEXT: &str = "ObjC.import('PDFKit'); function run(argv) { var d = $.PDFDocument.alloc.initWithURL($.NSURL.fileURLWithPath(argv[0])); return d.isNil() ? '' : d.string.js; }";

/// Blank-line paragraphs, each on one line.
fn paragraphs(t: &str) -> String {
    t.replace("\r\n", "\n")
        .split("\n\n")
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// PDF text breaks every line; a line ending a sentence (or a short line) ends a paragraph.
fn pdf_paragraphs(t: &str) -> String {
    let mut out = String::new();
    for line in t.lines().map(str::trim) {
        if line.is_empty() {
            out.push_str("\n\n");
            continue;
        }
        if let Some(stripped) = out.strip_suffix('-') {
            out = stripped.to_string();
        } else if !out.is_empty() && !out.ends_with("\n\n") {
            out.push(' ');
        }
        out.push_str(line);
        if line.ends_with(['.', '!', '?', ':', '”', '"']) && line.chars().count() < 60 {
            out.push_str("\n\n");
        }
    }
    paragraphs(&out)
}

/// Markdown read for its words: headings kept as "# ", code and front matter skipped, links and emphasis unwrapped.
pub fn markdown(src: &str) -> String {
    let mut out: Vec<String> = vec![];
    let mut para: Vec<String> = vec![];
    let (mut fence, mut front) = (false, false);
    let end = |para: &mut Vec<String>, out: &mut Vec<String>| {
        if !para.is_empty() {
            out.push(para.join(" "));
            para.clear();
        }
    };
    for (n, raw) in src.replace("\r\n", "\n").lines().enumerate() {
        let line = raw.trim();
        if n == 0 && line == "---" {
            front = true;
            continue;
        }
        if front {
            front = line != "---";
            continue;
        }
        if line.starts_with("```") || line.starts_with("~~~") {
            fence = !fence;
            end(&mut para, &mut out);
            continue;
        }
        if fence {
            continue;
        }
        if line.is_empty() || line.chars().all(|c| matches!(c, '-' | '*' | '_' | '=' | ' ')) && line.len() >= 3 {
            end(&mut para, &mut out);
            continue;
        }
        if let Some(h) = line.strip_prefix('#') {
            end(&mut para, &mut out);
            let h = inline(h.trim_start_matches('#').trim());
            if !h.is_empty() {
                out.push(format!("# {h}"));
            }
            continue;
        }
        let mut l = line.trim_start_matches('>').trim();
        let item = l.starts_with("- ") || l.starts_with("* ") || l.starts_with("+ ") || l.split_once(". ").is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        if item {
            end(&mut para, &mut out);
            l = l.split_once(' ').map_or(l, |(_, r)| r.trim());
        }
        if l.starts_with('|') {
            continue; // tables don't read aloud
        }
        let l = inline(l);
        if !l.is_empty() {
            para.push(l);
        }
        if item {
            end(&mut para, &mut out);
        }
    }
    end(&mut para, &mut out);
    out.join("\n\n")
}

/// Images dropped, links to their words, emphasis, code ticks and tags removed.
fn inline(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '!' if c.get(i + 1) == Some(&'[') => {
                // ![alt](src): nothing to say
                if let Some(close) = c[i..].iter().position(|&x| x == ')') {
                    i += close + 1;
                    continue;
                }
            }
            '[' => {
                if let Some(mid) = c[i..].iter().position(|&x| x == ']') {
                    let words: String = c[i + 1..i + mid].iter().collect();
                    let after = i + mid + 1;
                    let skip = if c.get(after) == Some(&'(') { c[after..].iter().position(|&x| x == ')').map(|p| after + p + 1) } else { Some(after) };
                    if let Some(next) = skip {
                        out.push_str(&words);
                        i = next;
                        continue;
                    }
                }
            }
            '<' => {
                if let Some(close) = c[i..].iter().position(|&x| x == '>') {
                    i += close + 1;
                    continue;
                }
            }
            '*' | '`' => {
                i += 1;
                continue;
            }
            '_' if (i == 0 || !c[i - 1].is_alphanumeric()) || c.get(i + 1).is_none_or(|n| !n.is_alphanumeric()) => {
                i += 1;
                continue;
            }
            _ => {}
        }
        out.push(c[i]);
        i += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_keep_sentences_and_headings() {
        let text = "# Opening\n\nOne. Two three.\n\nA second paragraph here.\n\n# Next part\n\nLast words.";
        let p = split(text, 4000);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].heading.as_deref(), Some("Opening"));
        assert!(p[0].text.starts_with("Opening.\n\nOne. Two three."));
        assert_eq!(p[1].heading.as_deref(), Some("Next part"));

        let long = (0..50).map(|i| format!("Sentence number {i} is here.")).collect::<Vec<_>>().join(" ");
        let p = split(&long, 200);
        assert!(p.len() > 5);
        for x in &p {
            assert!(x.text.chars().count() <= 200, "{} chars", x.text.len());
            assert!(x.text.ends_with('.'), "cut mid-sentence: {}", x.text);
        }
        assert_eq!(p.iter().map(|x| x.text.clone()).collect::<Vec<_>>().join(" "), long, "nothing lost");
    }

    #[test]
    fn a_sentence_longer_than_a_piece_is_cut_between_words() {
        let s = "word ".repeat(100);
        let p = split(&s, 50);
        assert!(p.iter().all(|x| x.text.chars().count() <= 50));
        assert_eq!(p.iter().map(|x| x.text.split(' ').count()).sum::<usize>(), 100);
    }

    #[test]
    fn sentences_end_at_capitals() {
        assert_eq!(sentences("It was 3 p.m. here. Then “Go!” he said. 2026 began."), vec!["It was 3 p.m. here.", "Then “Go!” he said.", "2026 began."]);
        assert_eq!(sentences("e.g. this stays whole"), vec!["e.g. this stays whole"]);
    }

    #[test]
    fn transcript_and_chapters_follow_the_pieces() {
        let p = split("# A\n\nOne two. Three four five six.\n\n# B\n\nSeven.", 25);
        let lengths: Vec<f64> = p.iter().map(|_| 2.0).collect();
        let (vtt, ch, total) = timings(&p, &lengths);
        assert!((total - (2.0 * p.len() as f64 + GAP * (p.len() - 1) as f64)).abs() < 1e-9);
        assert!(vtt.starts_with("WEBVTT\n\n00:00:00.000 --> "));
        let ch: serde_json::Value = serde_json::from_str(&ch.unwrap()).unwrap();
        assert_eq!(ch["chapters"].as_array().unwrap().len(), 2);
        assert_eq!(ch["chapters"][0]["startTime"], 0.0);
        // The second heading starts where its piece does.
        let b = p.iter().position(|x| x.heading.as_deref() == Some("B")).unwrap();
        assert!((ch["chapters"][1]["startTime"].as_f64().unwrap() - (b as f64 * (2.0 + GAP))).abs() < 1e-3);
        // Every cue ends where the next begins, and the last ends with the audio.
        let ends: Vec<&str> = vtt.lines().filter(|l| l.contains("-->")).collect();
        assert!(ends.last().unwrap().ends_with(&stamp(total)));
        assert!(timings(&split("No headings.", 100), &[1.0]).1.is_none());
    }

    #[test]
    fn markdown_reads_as_words() {
        let md = "---\ntitle: x\n---\n# Title\n\nSome **bold** and a [link](https://a.b) and ![img](i.png).\nSame paragraph.\n\n```\ncode()\n```\n\n- one\n- two\n\n| a | b |\n\n## Sub_heading\n\nsnake_case stays.";
        assert_eq!(markdown(md), "# Title\n\nSome bold and a link and . Same paragraph.\n\none\n\ntwo\n\n# Sub_heading\n\nsnake_case stays.");
    }

    #[test]
    fn joins_pieces_at_one_rate() {
        let dir = std::env::temp_dir().join(format!("zenpod-join-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let tone = |rate: u32, secs: f64| -> Vec<u8> {
            let n = (rate as f64 * secs) as usize;
            let pcm: Vec<u8> = (0..n).flat_map(|i| (((i as f32 * 0.05).sin() * 8000.0) as i16).to_le_bytes()).collect();
            voice::wav_from_l16(&pcm, rate)
        };
        let (a, b, out) = (dir.join("0.wav"), dir.join("1.wav"), dir.join("joined.wav"));
        std::fs::write(&a, tone(24000, 1.0)).unwrap();
        std::fs::write(&b, tone(22050, 0.5)).unwrap();
        let lengths = join(&[a, b], &out).unwrap();
        assert!((lengths[0] - 1.0).abs() < 0.01 && (lengths[1] - 0.5).abs() < 0.01, "{lengths:?}");
        let (pcm, rate) = decode_mono(&out).unwrap();
        assert_eq!(rate, 24000);
        assert!((pcm.len() as f64 / 24000.0 - (1.5 + GAP)).abs() < 0.01);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn this_mac_reads_and_afconvert_saves() {
        // Real `say` and `afconvert`, as the reader uses them; macOS only.
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let a = rt.block_on(voice::speak(&voice::Voice::Mac(String::new()), "Testing one two.")).unwrap();
        assert_eq!(a.ext, "wav");
        let dir = std::env::temp_dir().join(format!("zenpod-say-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (piece, wav, m4a) = (dir.join("0.wav"), dir.join("j.wav"), dir.join("o.m4a"));
        std::fs::write(&piece, &a.bytes).unwrap();
        let l = join(&[piece], &wav).unwrap();
        assert!(l[0] > 0.5, "{l:?}");
        let ok = std::process::Command::new("afconvert").args(["-f", "m4af", "-d", "aac", "-q", "127", "-s", "3"]).arg(&wav).arg(&m4a).status().unwrap();
        assert!(ok.success());
        let (pcm, _) = decode_mono(&m4a).unwrap();
        assert!(pcm.len() > 10_000);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
