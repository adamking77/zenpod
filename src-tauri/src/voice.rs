//! The voice that reads to you: this Mac's own voices through `say`, or a service with your key.
//! Each service is one small adapter: build the request, send it, return audio bytes.
//! Keys live in the macOS Keychain; everything else in the settings table.

use std::collections::HashMap;
use std::sync::LazyLock;

use base64::Engine;
use serde::Serialize;

use crate::store;

/// Services take a while on a long piece; podcast fetches keep their own 30s client.
static TTS: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder().timeout(std::time::Duration::from_secs(180)).build().expect("http client")
});

const KEYCHAIN: &str = "me.adamking.listener";
pub const SAMPLE: &str = "This is how Zenpod will read to you.";

pub struct Service {
    pub id: &'static str,
    pub name: &'static str,
    pub model: &'static str,
    pub voice: &'static str,
    /// Characters per request: the service's own limit, or Zenpod's 4,000 cap when that's lower.
    pub cap: usize,
}

pub const SERVICES: &[Service] = &[
    Service { id: "openai", name: "OpenAI", model: "gpt-4o-mini-tts", voice: "marin", cap: 4000 },
    Service { id: "gemini", name: "Google Gemini", model: "gemini-3.8-flash-tts", voice: "Kore", cap: 4000 },
    Service { id: "grok", name: "xAI Grok", model: "", voice: "eve", cap: 4000 },
    Service { id: "minimax", name: "MiniMax", model: "speech-2.8-hd", voice: "English_expressive_narrator", cap: 4000 },
    Service { id: "inworld", name: "Inworld", model: "inworld-tts-2", voice: "Ashley", cap: 2000 },
    Service { id: "elevenlabs", name: "ElevenLabs", model: "eleven_multilingual_v2", voice: "21m00Tcm4TlvDq8ikWAM", cap: 4000 },
    Service { id: "compat", name: "OpenAI-compatible", model: "kokoro", voice: "af_heart", cap: 4000 },
];

pub const COMPAT_ADDRESS: &str = "http://localhost:8880/v1";

pub fn service(id: &str) -> Option<&'static Service> {
    SERVICES.iter().find(|s| s.id == id)
}

/// Who reads, resolved from settings (and a show's own voice) at the moment of reading.
#[derive(Clone, Debug)]
pub enum Voice {
    /// A system voice by name; empty means the Mac's default.
    Mac(String),
    Service { id: &'static str, model: String, voice: String, key: Option<String>, address: String },
}

impl Voice {
    pub fn cap(&self) -> usize {
        match self {
            Voice::Mac(_) => 3000,
            Voice::Service { id, .. } => service(id).map_or(4000, |s| s.cap),
        }
    }
}

fn set(db: &rusqlite::Connection, k: &str) -> Option<String> {
    store::setting(db, k).filter(|v| !v.trim().is_empty())
}

/// The default voice from Settings, or a show's own ("mac:<name>") when it has one.
pub fn resolve(db: &rusqlite::Connection, show_voice: Option<&str>) -> Voice {
    if let Some(name) = show_voice.and_then(|v| v.strip_prefix("mac:")) {
        return Voice::Mac(name.to_string());
    }
    if set(db, "voice_from").as_deref() != Some("service") {
        return Voice::Mac(set(db, "voice_mac").unwrap_or_default());
    }
    let s = set(db, "voice_service").and_then(|id| service(&id)).unwrap_or(&SERVICES[0]);
    Voice::Service {
        id: s.id,
        model: set(db, &format!("voice_{}_model", s.id)).unwrap_or_else(|| s.model.into()),
        voice: set(db, &format!("voice_{}_voice", s.id)).unwrap_or_else(|| s.voice.into()),
        key: key(s.id),
        address: set(db, "voice_compat_address").unwrap_or_else(|| COMPAT_ADDRESS.into()),
    }
}

// ---------- the Keychain ----------

fn entry(id: &str) -> Option<keyring::Entry> {
    keyring::Entry::new(KEYCHAIN, &format!("voice-{id}")).ok()
}

pub fn key(id: &str) -> Option<String> {
    entry(id)?.get_password().ok().filter(|k| !k.trim().is_empty())
}

pub fn set_key(id: &str, value: &str) -> Result<(), String> {
    let e = entry(id).ok_or("The Keychain couldn't be opened.")?;
    if value.trim().is_empty() {
        let _ = e.delete_credential();
        Ok(())
    } else {
        e.set_password(value.trim()).map_err(|e| format!("The Keychain refused the key: {e}"))
    }
}

/// Which services have a key saved, from a flag kept beside the settings: the Keychain is only opened to read a key
/// when reading something, never to fill in Settings (each opening can ask the person's permission).
pub fn keys_saved(db: &rusqlite::Connection) -> HashMap<&'static str, bool> {
    SERVICES.iter().map(|s| (s.id, store::setting(db, &format!("voice_{}_keyed", s.id)).as_deref() == Some("1"))).collect()
}

// ---------- this Mac ----------

#[derive(Serialize, Clone)]
pub struct MacVoice {
    pub name: String,
    pub locale: String,
}

/// `say -v '?'`: "Eddy (English (US))   en_US    # Hello! My name is Eddy." English first, then the rest.
pub fn mac_voices() -> Vec<MacVoice> {
    let Ok(out) = std::process::Command::new("say").args(["-v", "?"]).output() else { return vec![] };
    let mut v: Vec<MacVoice> = String::from_utf8_lossy(&out.stdout).lines().filter_map(parse_voice_line).collect();
    // A voice installed in two qualities is listed twice under one name; `say -v` takes the name, so list it once.
    let mut seen = std::collections::HashSet::new();
    v.retain(|m| seen.insert(m.name.clone()));
    v.sort_by_key(|m| !m.locale.starts_with("en"));
    v
}

fn parse_voice_line(line: &str) -> Option<MacVoice> {
    let head = line.split(" # ").next()?.trim_end();
    let (name, locale) = head.rsplit_once(char::is_whitespace)?;
    let name = name.trim();
    (!name.is_empty() && locale.contains('_')).then(|| MacVoice { name: name.into(), locale: locale.into() })
}

// ---------- reading ----------

/// Audio for one piece of text, with the extension that says how to decode it.
pub struct Audio {
    pub bytes: Vec<u8>,
    pub ext: &'static str,
}

pub async fn speak(v: &Voice, text: &str) -> Result<Audio, String> {
    match v {
        Voice::Mac(name) => mac(name, text).await,
        Voice::Service { id, model, voice, key, address } => {
            let name = service(id).map_or("The service", |s| s.name);
            if key.is_none() && *id != "compat" {
                return Err(format!("{name} needs a key. Paste it in Settings\u{a0}›\u{a0}Voice."));
            }
            let k = key.clone().unwrap_or_default();
            let req = match *id {
                "openai" => openai_like(TTS.post("https://api.openai.com/v1/audio/speech"), &k, model, voice, text),
                "compat" => openai_like(TTS.post(format!("{}/audio/speech", address.trim_end_matches('/'))), &k, model, voice, text),
                "gemini" => TTS.post("https://generativelanguage.googleapis.com/v1beta/interactions").header("x-goog-api-key", &k).json(&serde_json::json!({
                    "model": model,
                    "input": [{ "type": "user_input", "content": [{ "type": "text", "text": text }] }],
                    "response_format": { "type": "audio", "mime_type": "audio/l16", "sample_rate": 24000 },
                    "generation_config": { "speech_config": [{ "voice": voice }] },
                    "stream": false,
                })),
                "grok" => TTS.post("https://api.x.ai/v1/tts").bearer_auth(&k).json(&serde_json::json!({ "text": text, "voice_id": voice, "language": "auto" })),
                "minimax" => TTS.post("https://api.minimax.io/v1/t2a_v2").bearer_auth(&k).json(&serde_json::json!({
                    "model": model, "text": text,
                    "voice_setting": { "voice_id": voice },
                    "audio_setting": { "format": "mp3", "sample_rate": 32000 },
                })),
                "inworld" => TTS.post("https://api.inworld.ai/tts/v1/voice").header("Authorization", format!("Basic {k}")).json(&serde_json::json!({
                    "text": text, "voiceId": voice, "modelId": model, "audioConfig": { "audioEncoding": "MP3" },
                })),
                "elevenlabs" => TTS
                    .post(format!("https://api.elevenlabs.io/v1/text-to-speech/{voice}?output_format=mp3_44100_128"))
                    .header("xi-api-key", &k)
                    .json(&serde_json::json!({ "text": text, "model_id": model })),
                _ => return Err("That service isn't known.".into()),
            };
            let res = req.send().await.map_err(|e| unreachable(id, address, &e))?;
            let status = res.status().as_u16();
            let body = res.bytes().await.map_err(|e| unreachable(id, address, &e))?.to_vec();
            if !(200..300).contains(&status) {
                return Err(refused(name, status, &body, model, voice));
            }
            decode_body(id, body, name, model, voice)
        }
    }
}

fn openai_like(r: reqwest::RequestBuilder, key: &str, model: &str, voice: &str, text: &str) -> reqwest::RequestBuilder {
    let r = if key.is_empty() { r } else { r.bearer_auth(key) };
    r.json(&serde_json::json!({ "model": model, "input": text, "voice": voice, "response_format": "mp3" }))
}

/// Unwrap each service's way of returning audio.
fn decode_body(id: &str, body: Vec<u8>, name: &str, model: &str, voice: &str) -> Result<Audio, String> {
    let b64 = |s: &str| base64::engine::general_purpose::STANDARD.decode(s.trim()).map_err(|_| format!("{name} sent audio Zenpod couldn't read."));
    let json = || serde_json::from_slice::<serde_json::Value>(&body).map_err(|_| format!("{name} sent something that isn't audio."));
    match id {
        "gemini" => {
            let j = json()?;
            let data = j["steps"].as_array().into_iter().flatten()
                .flat_map(|s| s["content"].as_array().into_iter().flatten())
                .find_map(|c| c["data"].as_str())
                .ok_or_else(|| format!("{name} sent no audio."))?;
            let raw = b64(data)?;
            // Unary replies are WAV; a bare L16 stream is wrapped so it decodes the same way.
            Ok(if raw.starts_with(b"RIFF") { Audio { bytes: raw, ext: "wav" } } else { Audio { bytes: wav_from_l16(&raw, 24000), ext: "wav" } })
        }
        "minimax" => {
            let j = json()?;
            if j["base_resp"]["status_code"].as_i64().unwrap_or(0) != 0 {
                let msg = j["base_resp"]["status_msg"].as_str().unwrap_or("").to_string();
                return Err(refused(name, 400, msg.as_bytes(), model, voice));
            }
            let hex = j["data"]["audio"].as_str().ok_or_else(|| format!("{name} sent no audio."))?;
            Ok(Audio { bytes: from_hex(hex).ok_or_else(|| format!("{name} sent audio Zenpod couldn't read."))?, ext: "mp3" })
        }
        "inworld" => {
            let j = json()?;
            Ok(Audio { bytes: b64(j["audioContent"].as_str().ok_or_else(|| format!("{name} sent no audio."))?)?, ext: "mp3" })
        }
        _ => {
            if body.first() == Some(&b'{') {
                return Err(format!("{name} sent something that isn't audio."));
            }
            Ok(Audio { bytes: body, ext: "mp3" })
        }
    }
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    s.len().is_multiple_of(2).then_some(())?;
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// 16-bit little-endian mono PCM, as a WAV file.
pub fn wav_from_l16(pcm: &[u8], rate: u32) -> Vec<u8> {
    let mut w = wav_header(rate, pcm.len() as u32);
    w.extend_from_slice(pcm);
    w
}

pub fn wav_header(rate: u32, data_len: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(44);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&(36 + data_len).to_le_bytes());
    h.extend_from_slice(b"WAVEfmt ");
    h.extend_from_slice(&16u32.to_le_bytes());
    h.extend_from_slice(&1u16.to_le_bytes()); // PCM
    h.extend_from_slice(&1u16.to_le_bytes()); // mono
    h.extend_from_slice(&rate.to_le_bytes());
    h.extend_from_slice(&(rate * 2).to_le_bytes());
    h.extend_from_slice(&2u16.to_le_bytes());
    h.extend_from_slice(&16u16.to_le_bytes());
    h.extend_from_slice(b"data");
    h.extend_from_slice(&data_len.to_le_bytes());
    h
}

fn unreachable(id: &str, address: &str, e: &reqwest::Error) -> String {
    if e.is_timeout() {
        return format!("{} took too long to answer. Try again.", service(id).map_or("The service", |s| s.name));
    }
    let host = if id == "compat" { address.to_string() } else { service(id).map_or("the service".into(), |s| s.name.to_string()) };
    format!("Couldn't reach {host}. Is it running?")
}

/// One line: what happened, and what to do.
fn refused(name: &str, status: u16, body: &[u8], model: &str, voice: &str) -> String {
    let text = String::from_utf8_lossy(body).to_lowercase();
    let says = |w: &str| text.contains(w);
    match status {
        401 | 403 => format!("{name} didn't accept the key. Check it in Settings\u{a0}›\u{a0}Voice."),
        402 | 429 => format!("{name} says you've hit your limit. Try again later or check your account."),
        _ if says("voice") && (says("not found") || says("invalid") || says("unknown") || says("does not exist")) => format!("{name} doesn't know the voice “{voice}”."),
        404 => format!("{name} doesn't know the model “{model}”."),
        _ if says("model") && (says("not found") || says("invalid") || says("unknown") || says("does not exist")) => format!("{name} doesn't know the model “{model}”."),
        _ if says("quota") || says("insufficient") || says("credit") || says("balance") => format!("{name} says you've hit your limit. Try again later or check your account."),
        _ if says("api key") || says("unauthorized") || says("authentication") => format!("{name} didn't accept the key. Check it in Settings\u{a0}›\u{a0}Voice."),
        _ => {
            let raw = String::from_utf8_lossy(body);
            let msg = serde_json::from_slice::<serde_json::Value>(body)
                .ok()
                .and_then(|j| ["error.message", "message", "detail.message", "error", "detail", "base_resp.status_msg"].iter().find_map(|p| {
                    let v = p.split('.').fold(&j, |v, k| &v[k]);
                    v.as_str().map(str::to_string)
                }))
                .unwrap_or_else(|| raw.to_string());
            let one = msg.lines().next().unwrap_or("").trim();
            let one: String = one.chars().take(140).collect();
            if one.is_empty() { format!("{name} couldn't read this ({status}).") } else { format!("{name}: {one}") }
        }
    }
}

/// `say` writes 16-bit WAV, which decodes like any other piece.
async fn mac(name: &str, text: &str) -> Result<Audio, String> {
    let dir = std::env::temp_dir().join(format!("zenpod-say-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let n = SAY_N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (txt, wav) = (dir.join(format!("{n}.txt")), dir.join(format!("{n}.wav")));
    std::fs::write(&txt, text).map_err(|e| e.to_string())?;
    let mut cmd = tokio::process::Command::new("say");
    if !name.is_empty() {
        cmd.args(["-v", name]);
    }
    let out = cmd.arg("-f").arg(&txt).arg("-o").arg(&wav).arg("--data-format=LEI16@22050").output().await.map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&txt);
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if why.to_lowercase().contains("voice") { format!("This Mac doesn't have the voice “{name}”.") } else { format!("This Mac couldn't read it: {why}") });
    }
    let bytes = std::fs::read(&wav).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&wav);
    Ok(Audio { bytes, ext: "wav" })
}

static SAY_N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_lines() {
        let v = parse_voice_line("Eddy (English (US))   en_US    # Hello! My name is Eddy.").unwrap();
        assert_eq!((v.name.as_str(), v.locale.as_str()), ("Eddy (English (US))", "en_US"));
        let v = parse_voice_line("Samantha            en_US    # Hello! My name is Samantha.").unwrap();
        assert_eq!(v.name, "Samantha");
        assert!(parse_voice_line("").is_none());
    }

    #[test]
    fn mac_voices_are_listed_once() {
        // Real `say -v '?'`: this Mac lists some voices twice (two qualities, one name); menus key on the name.
        let v = mac_voices();
        let mut names: Vec<&str> = v.iter().map(|m| m.name.as_str()).collect();
        let n = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), n, "a voice name is listed twice");
    }

    #[test]
    fn hex_and_wav() {
        assert_eq!(from_hex("00ff10"), Some(vec![0, 255, 16]));
        assert_eq!(from_hex("abc"), None);
        let w = wav_from_l16(&[0, 0, 1, 0], 24000);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(w.len(), 48);
    }

    #[test]
    fn errors_say_what_to_do() {
        assert!(refused("OpenAI", 401, b"{}", "m", "v").contains("didn't accept the key"));
        assert!(refused("OpenAI", 429, b"{}", "m", "v").contains("hit your limit"));
        assert!(refused("OpenAI", 404, b"{}", "tts-9", "v").contains("model “tts-9”"));
        assert!(refused("ElevenLabs", 400, br#"{"detail":{"message":"voice_id not found"}}"#, "m", "abc").contains("voice “abc”"));
        assert_eq!(refused("Grok", 500, br#"{"error":{"message":"Server busy\nretry"}}"#, "m", "v"), "Grok: Server busy");
    }
}
