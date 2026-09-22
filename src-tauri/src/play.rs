//! Playback state lives here. The hidden player window owns the <audio> element and does what
//! this module tells it; every visible window renders the `state` event. State flows one way.

use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::{err, feed::HTTP, store, Core, R};

#[derive(Serialize, Clone)]
pub struct Now {
    pub episode: Option<store::EpisodeRow>,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
    pub speed: f64,
    #[serde(skip)]
    saved_at: Option<Instant>,
}

impl Default for Now {
    fn default() -> Self {
        Now { episode: None, playing: false, position: 0.0, duration: 0.0, speed: 1.0, saved_at: None }
    }
}

#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Cmd {
    Load { src: String, position: f64, speed: f64, play: bool, title: String, show: String, art: String },
    Play,
    Pause,
    Seek { position: f64 },
    Speed { speed: f64 },
}

fn send(app: &AppHandle, cmd: Cmd) {
    let _ = app.emit_to("player", "cmd", cmd);
}

fn broadcast(app: &AppHandle, now: &Now) {
    let _ = app.emit("state", now);
}

fn load_cmd(now: &Now, play: bool) -> Option<Cmd> {
    let e = now.episode.as_ref()?;
    Some(Cmd::Load {
        src: format!("listener://localhost/audio/{}", e.id),
        position: now.position,
        speed: now.speed,
        play,
        title: e.title.clone(),
        show: e.show_title.clone(),
        art: format!("listener://localhost/art/{}", e.show_id),
    })
}

/// The hidden window that plays. It never closes, so playback survives every visible window.
pub fn spawn_player(app: &AppHandle) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, "player", WebviewUrl::App("player".into()))
        .visible(false)
        .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
        .build()?;
    Ok(())
}

/// Pick up where the person left off: the last episode, paused, at its saved position.
pub fn restore(app: &AppHandle) {
    let core = app.state::<Core>();
    let db = core.db.lock().unwrap();
    let mut now = core.now.lock().unwrap();
    now.speed = store::setting(&db, "speed").and_then(|s| s.parse().ok()).unwrap_or(1.0);
    if let Some(e) = store::setting(&db, "current").and_then(|id| id.parse().ok()).and_then(|id| store::episode(&db, id).ok().flatten()) {
        now.position = e.position;
        now.duration = e.duration.unwrap_or(0.0);
        now.episode = Some(e);
    }
}

fn persist(db: &rusqlite::Connection, now: &mut Now) {
    if let Some(e) = &now.episode {
        let _ = store::save_position(db, e.id, now.position);
        now.saved_at = Some(Instant::now());
    }
}

#[tauri::command]
pub fn playback(core: State<Core>) -> Now {
    core.now.lock().unwrap().clone()
}

#[tauri::command]
pub fn player_ready(app: AppHandle, core: State<Core>) {
    if let Some(cmd) = load_cmd(&core.now.lock().unwrap(), false) {
        send(&app, cmd);
    }
}

/// Choosing an episode loads it, paused, at its saved position. Play starts it.
#[tauri::command]
pub fn choose(app: AppHandle, core: State<Core>, id: i64) -> R<()> {
    let db = core.db.lock().unwrap();
    let mut now = core.now.lock().unwrap();
    if now.episode.as_ref().is_some_and(|e| e.id == id) {
        return Ok(());
    }
    persist(&db, &mut now);
    let e = store::episode(&db, id).map_err(err)?.ok_or("That episode is gone.")?;
    now.position = if e.played { 0.0 } else { e.position };
    now.duration = e.duration.unwrap_or(0.0);
    now.playing = false;
    now.episode = Some(e);
    store::set_setting(&db, "current", &id.to_string()).map_err(err)?;
    if let Some(cmd) = load_cmd(&now, false) {
        send(&app, cmd);
    }
    broadcast(&app, &now);
    Ok(())
}

#[tauri::command]
pub fn toggle(app: AppHandle, core: State<Core>) {
    let started = {
        let db = core.db.lock().unwrap();
        let mut now = core.now.lock().unwrap();
        let Some(id) = now.episode.as_ref().map(|e| e.id) else { return };
        now.playing = !now.playing;
        send(&app, if now.playing { Cmd::Play } else { Cmd::Pause });
        if !now.playing {
            persist(&db, &mut now);
        }
        broadcast(&app, &now);
        now.playing.then_some(id)
    };
    if let Some(id) = started {
        cache(app, id);
    }
}

#[tauri::command]
pub fn seek(app: AppHandle, core: State<Core>, position: f64) {
    let db = core.db.lock().unwrap();
    let mut now = core.now.lock().unwrap();
    if now.episode.is_none() {
        return;
    }
    let max = if now.duration > 0.0 { now.duration } else { f64::MAX };
    now.position = position.clamp(0.0, max);
    send(&app, Cmd::Seek { position: now.position });
    persist(&db, &mut now);
    broadcast(&app, &now);
}

#[tauri::command]
pub fn skip(app: AppHandle, core: State<Core>, by: f64) {
    let to = core.now.lock().unwrap().position + by;
    seek(app, core, to);
}

#[tauri::command]
pub fn set_speed(app: AppHandle, core: State<Core>, speed: f64) -> R<()> {
    let db = core.db.lock().unwrap();
    let mut now = core.now.lock().unwrap();
    now.speed = speed.clamp(0.5, 3.0);
    store::set_setting(&db, "speed", &now.speed.to_string()).map_err(err)?;
    send(&app, Cmd::Speed { speed: now.speed });
    broadcast(&app, &now);
    Ok(())
}

/// The player window reports what the audio element is actually doing.
#[tauri::command]
pub fn report(app: AppHandle, core: State<Core>, id: i64, position: f64, duration: f64, playing: bool, ended: bool) {
    let db = core.db.lock().unwrap();
    let mut now = core.now.lock().unwrap();
    let Some(e) = now.episode.as_mut() else { return };
    if e.id != id {
        return; // a late report from the previous episode
    }
    if duration.is_finite() && duration > 0.0 && (e.duration.unwrap_or(0.0) - duration).abs() > 1.0 {
        e.duration = Some(duration);
        let _ = store::set_duration(&db, id, duration);
    }
    if duration.is_finite() && duration > 0.0 {
        now.duration = duration;
    }
    now.position = position;
    let was_playing = now.playing;
    now.playing = playing && !ended;
    if ended {
        let _ = store::mark_played(&db, id);
        if let Some(e) = now.episode.as_mut() {
            e.played = true;
        }
        now.position = 0.0;
        let _ = app.emit("episode", id);
    } else if was_playing != now.playing || now.saved_at.is_none_or(|t| t.elapsed().as_secs() >= 5) {
        persist(&db, &mut now);
    }
    broadcast(&app, &now);
}

/// Keep a copy of what's playing on disk: it plays offline from then on, and M3 reads its loudness.
pub fn cache(app: AppHandle, id: i64) {
    {
        let core = app.state::<Core>();
        let db = core.db.lock().unwrap();
        let (local, _) = match store::audio_source(&db, id) {
            Ok(s) => s,
            Err(_) => return,
        };
        if local.is_some_and(|p| std::path::Path::new(&p).exists()) || !core.fetching.lock().unwrap().insert(id) {
            return;
        }
    }
    tauri::async_runtime::spawn(async move {
        let res = download(&app, id).await;
        app.state::<Core>().fetching.lock().unwrap().remove(&id);
        if let Err(e) = res {
            eprintln!("[download] episode {id}: {e}");
        }
        let _ = app.emit("episode", id);
    });
}

async fn download(app: &AppHandle, id: i64) -> Result<(), String> {
    use std::io::Write;
    let remote = store::audio_source(&app.state::<Core>().db.lock().unwrap(), id).map_err(err)?.1;
    let mut r = HTTP.get(&remote).send().await.and_then(|r| r.error_for_status()).map_err(err)?;
    let ext = match r.headers().get("content-type").and_then(|v| v.to_str().ok()) {
        Some(t) if t.contains("mp4") || t.contains("m4a") || t.contains("aac") => "m4a",
        _ => "mp3",
    };
    let dir = app.path().app_data_dir().map_err(err)?.join("audio");
    std::fs::create_dir_all(&dir).map_err(err)?;
    let (part, done) = (dir.join(format!("{id}.part")), dir.join(format!("{id}.{ext}")));
    let mut f = std::fs::File::create(&part).map_err(err)?;
    while let Some(chunk) = r.chunk().await.map_err(err)? {
        f.write_all(&chunk).map_err(err)?;
    }
    f.sync_all().map_err(err)?;
    std::fs::rename(&part, &done).map_err(err)?;
    let core = app.state::<Core>();
    store::set_local(&core.db.lock().unwrap(), id, Some(&done.to_string_lossy())).map_err(err)?;
    Ok(())
}
