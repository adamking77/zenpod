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
    /// Whether the list it was chosen from has an episode before and after it.
    pub prev: bool,
    pub next: bool,
    /// The list the episode was chosen from, in its order: previous, next and autoplay walk it.
    #[serde(skip)]
    queue: Vec<i64>,
    #[serde(skip)]
    saved_at: Option<Instant>,
}

impl Default for Now {
    fn default() -> Self {
        Now { episode: None, playing: false, position: 0.0, duration: 0.0, speed: 1.0, prev: false, next: false, queue: vec![], saved_at: None }
    }
}

impl Now {
    fn at(&self) -> Option<usize> {
        let id = self.episode.as_ref()?.id;
        self.queue.iter().position(|q| *q == id)
    }
    /// The episode `by` steps along the queue from this one.
    fn beside(&self, by: isize) -> Option<i64> {
        let i = self.at()?.checked_add_signed(by)?;
        self.queue.get(i).copied()
    }
    fn place(&mut self) {
        self.prev = self.beside(-1).is_some();
        self.next = self.beside(1).is_some();
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
    now.queue = store::setting(&db, "queue").unwrap_or_default().split(',').filter_map(|n| n.parse().ok()).collect();
    now.place();
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

/// Choosing an episode plays it from its saved position; choosing the one already loaded resumes it.
/// `queue` is the list it was chosen from. If the player can't start, it reports back and the state returns to paused.
#[tauri::command]
pub fn choose(app: AppHandle, core: State<Core>, id: i64, queue: Option<Vec<i64>>) -> R<()> {
    let db = core.db.lock().unwrap();
    let mut now = core.now.lock().unwrap();
    if let Some(q) = queue {
        set_queue(&db, &mut now, q)?;
    }
    if now.episode.as_ref().is_some_and(|e| e.id == id) {
        now.place();
        if !now.playing {
            now.playing = true;
            send(&app, Cmd::Play);
        }
        broadcast(&app, &now);
        return Ok(());
    }
    start(&app, &db, &mut now, id)
}

fn set_queue(db: &rusqlite::Connection, now: &mut Now, q: Vec<i64>) -> R<()> {
    let joined = q.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
    store::set_setting(db, "queue", &joined).map_err(err)?;
    now.queue = q;
    Ok(())
}

/// The list on screen becomes the queue when the playing episode is in it: previous, next and autoplay follow
/// what you're looking at, not only where you started.
#[tauri::command]
pub fn follow_list(app: AppHandle, core: State<Core>, queue: Vec<i64>) -> R<()> {
    let db = core.db.lock().unwrap();
    let mut now = core.now.lock().unwrap();
    let playing = now.episode.as_ref().map(|e| e.id);
    if playing.is_none_or(|id| !queue.contains(&id)) || now.queue == queue {
        return Ok(());
    }
    set_queue(&db, &mut now, queue)?;
    now.place();
    broadcast(&app, &now);
    Ok(())
}

fn start(app: &AppHandle, db: &rusqlite::Connection, now: &mut Now, id: i64) -> R<()> {
    persist(db, now);
    let e = store::episode(db, id).map_err(err)?.ok_or("That episode is gone.")?;
    now.position = if e.played { 0.0 } else { e.position };
    now.duration = e.duration.unwrap_or(0.0);
    now.playing = true;
    now.episode = Some(e);
    now.place();
    store::set_setting(db, "current", &id.to_string()).map_err(err)?;
    if let Some(cmd) = load_cmd(now, true) {
        send(app, cmd);
    }
    broadcast(app, now);
    Ok(())
}

/// Previous (-1) or next (1) in the list the episode was chosen from.
#[tauri::command]
pub fn step(app: AppHandle, core: State<Core>, by: isize) -> R<()> {
    let db = core.db.lock().unwrap();
    let mut now = core.now.lock().unwrap();
    match now.beside(by.signum()) {
        Some(id) => start(&app, &db, &mut now, id),
        None => Ok(()),
    }
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
        // Autoplay: the next episode down the list that hasn't been heard.
        if store::setting(&db, "autoplay").as_deref() != Some("off") {
            let after = now.at().map_or(&[][..], |i| &now.queue[i + 1..]);
            let next = after.iter().copied().find(|q| store::episode(&db, *q).ok().flatten().is_some_and(|e| !e.played));
            if let Some(next) = next {
                let _ = start(&app, &db, &mut now, next);
                return;
            }
        }
    } else if was_playing != now.playing || now.saved_at.is_none_or(|t| t.elapsed().as_secs() >= 5) {
        persist(&db, &mut now);
    }
    broadcast(&app, &now);
}

/// The episode's loudness shape, or nothing yet. Computing it is kicked off once the file is on disk.
#[tauri::command]
pub fn peaks(app: AppHandle, core: State<Core>, id: i64) -> Option<Vec<f32>> {
    let db = core.db.lock().unwrap();
    if let Some(b) = store::peaks(&db, id).ok().flatten() {
        return Some(crate::peaks::from_blob(&b));
    }
    let local = store::audio_source(&db, id).ok()?.0?;
    drop(db);
    measure(app, id, local.into());
    None
}

fn measure(app: AppHandle, id: i64, path: std::path::PathBuf) {
    if !app.state::<Core>().fetching.lock().unwrap().insert(-id) {
        return; // already measuring (negative ids mark measuring, positive downloading)
    }
    tauri::async_runtime::spawn_blocking(move || {
        let res = crate::peaks::compute(&path);
        let core = app.state::<Core>();
        core.fetching.lock().unwrap().remove(&-id);
        match res {
            Ok(p) => {
                let _ = store::set_peaks(&core.db.lock().unwrap(), id, &crate::peaks::to_blob(&p));
                let _ = app.emit("peaks", id);
            }
            Err(e) => eprintln!("[peaks] episode {id}: {e}"),
        }
    });
}

#[derive(Serialize, Clone)]
pub struct Chapter {
    title: String,
    start: f64,
}

/// A feed-linked file (chapters, transcript) fetched once and kept on disk so it works offline.
async fn kept_text(app: &AppHandle, kind: &str, id: i64, url: Option<String>) -> Option<String> {
    let dir = app.path().app_data_dir().ok()?.join(kind);
    let file = dir.join(format!("{id}.txt"));
    if let Ok(t) = std::fs::read_to_string(&file) {
        return Some(t);
    }
    let t = async { HTTP.get(url?).send().await.ok()?.error_for_status().ok()?.text().await.ok() }.await?;
    let _ = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, &t));
    Some(t)
}

/// Podcast Namespace chapters, when the feed has them.
#[tauri::command]
pub async fn chapters(app: AppHandle, id: i64) -> Vec<Chapter> {
    let url = store::chapters_url(&app.state::<Core>().db.lock().unwrap(), id).ok().flatten();
    kept_text(&app, "chapters", id, url).await.map(|t| parse_chapters(&t)).unwrap_or_default()
}

/// The episode's transcript as published (WebVTT, SRT, JSON or text); the page reads it.
#[tauri::command]
pub async fn transcript(app: AppHandle, id: i64) -> Option<String> {
    let url = store::transcript_url(&app.state::<Core>().db.lock().unwrap(), id).ok().flatten()?;
    kept_text(&app, "transcripts", id, Some(url)).await
}

/// Chapters meant for the table of contents (`toc: false` ones are skipped), in file order.
fn parse_chapters(text: &str) -> Vec<Chapter> {
    #[derive(serde::Deserialize)]
    struct File {
        chapters: Vec<Raw>,
    }
    #[derive(serde::Deserialize)]
    struct Raw {
        #[serde(default)]
        title: String,
        #[serde(rename = "startTime")]
        start: f64,
        #[serde(default = "yes")]
        toc: bool,
    }
    fn yes() -> bool {
        true
    }
    serde_json::from_str::<File>(text)
        .map(|f| f.chapters.into_iter().filter(|c| c.toc).map(|c| Chapter { title: c.title, start: c.start }).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn chapters() {
        let f = r#"{"version":"1.2.0","chapters":[{"title":"EP 489","startTime":0,"endTime":4},
          {"title":"Time Flies","startTime":298.5},{"title":"hidden","startTime":300,"toc":false}]}"#;
        let c = super::parse_chapters(f);
        assert_eq!(c.len(), 2);
        assert_eq!((c[1].title.as_str(), c[1].start), ("Time Flies", 298.5));
        assert!(super::parse_chapters("not json").is_empty());
    }

    #[test]
    fn queue_edges() {
        let ep = |id| crate::store::EpisodeRow {
            id, show_id: 1, show_title: String::new(), title: String::new(), published: None, duration: None,
            position: 0.0, played: false, kept: false, offline: false, image_url: None,
        };
        let mut n = super::Now { queue: vec![5, 6, 7], episode: Some(ep(5)), ..Default::default() };
        n.place();
        assert_eq!((n.prev, n.next, n.beside(1)), (false, true, Some(6)));
        n.episode = Some(ep(7));
        n.place();
        assert_eq!((n.prev, n.next, n.beside(-1)), (true, false, Some(6)));
        // An episode outside the list has neither.
        n.episode = Some(ep(9));
        n.place();
        assert_eq!((n.prev, n.next), (false, false));
    }
}

/// Keep an episode offline, or let it go.
#[tauri::command]
pub fn keep(app: AppHandle, core: State<Core>, id: i64, on: bool) -> R<()> {
    let current = core.now.lock().unwrap().episode.as_ref().map(|e| e.id);
    {
        let db = core.db.lock().unwrap();
        store::set_kept(&db, id, on).map_err(err)?;
        if !on && current != Some(id) {
            if let Some(p) = store::audio_source(&db, id).map_err(err)?.0 {
                let _ = std::fs::remove_file(p);
            }
            store::set_local(&db, id, None).map_err(err)?;
        }
    }
    let _ = app.emit("episode", id);
    if on {
        cache(app, id);
    }
    Ok(())
}

/// On launch: remove saved copies of heard episodes that weren't kept.
pub fn tidy(app: &AppHandle) {
    let core = app.state::<Core>();
    let db = core.db.lock().unwrap();
    let current = core.now.lock().unwrap().episode.as_ref().map(|e| e.id);
    for (id, path) in store::spent_copies(&db).unwrap_or_default() {
        if Some(id) != current {
            let _ = std::fs::remove_file(&path);
            let _ = store::set_local(&db, id, None);
        }
    }
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
    measure(app.clone(), id, done);
    Ok(())
}
