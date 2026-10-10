mod feed;
mod marks;
mod modes;
mod peaks;
mod play;
mod proto;
mod read;
mod sources;
mod store;
mod voice;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};

pub struct Core {
    db: Mutex<Connection>,
    now: Mutex<play::Now>,
    /// Episodes being saved to disk right now.
    fetching: Mutex<HashSet<i64>>,
    /// Enclosure addresses after their tracking redirects.
    urls: Mutex<HashMap<i64, String>>,
    /// Files opened with Zenpod and zenpod://read links, held until the window takes them (it may not have loaded yet).
    incoming: Mutex<Vec<Incoming>>,
}

#[derive(Serialize, Clone)]
struct Incoming {
    /// "file" (a path) or "link" (an address to read)
    kind: &'static str,
    value: String,
    /// The show it should go into, when the link names one.
    show: Option<String>,
}

type R<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn library_changed(app: &AppHandle) {
    let _ = app.emit("library", ());
}

/// Fetch one feed and store it. Returns the show's id and title.
async fn ingest(app: &AppHandle, url: &str) -> R<(i64, String)> {
    let (show, eps) = feed::fetch(url).await?;
    let core = app.state::<Core>();
    let mut db = core.db.lock().unwrap();
    let id = store::upsert_show(&db, url, &show).map_err(err)?;
    store::upsert_episodes(&mut db, id, &eps).map_err(err)?;
    Ok((id, show.title))
}

/// Run `f` over `items` with at most `n` in flight.
async fn bounded<T, F, Fut, O>(items: Vec<T>, n: usize, f: F) -> Vec<O>
where
    T: Send + 'static,
    O: Send + 'static,
    F: Fn(T) -> Fut,
    Fut: std::future::Future<Output = O> + Send + 'static,
{
    let gate = Arc::new(tokio::sync::Semaphore::new(n));
    let tasks: Vec<_> = items
        .into_iter()
        .map(|it| {
            let gate = gate.clone();
            let fut = f(it);
            tauri::async_runtime::spawn(async move {
                let _p = gate.acquire_owned().await;
                fut.await
            })
        })
        .collect();
    let mut out = Vec::with_capacity(tasks.len());
    for t in tasks {
        if let Ok(o) = t.await {
            out.push(o);
        }
    }
    out
}

#[tauri::command]
async fn add_show(app: AppHandle, input: String) -> R<(i64, String)> {
    let input = input.trim();
    let url = if let Some(id) = feed::apple_id(input) {
        feed::lookup_feed(id)
            .await
            .ok_or("Apple doesn't share a public feed for that show.")?
    } else if input.starts_with("http://") || input.starts_with("https://") {
        input.to_string()
    } else {
        feed::find_feed(input, "")
            .await
            .ok_or_else(|| format!("No public feed was found for “{input}”."))?
    };
    let added = ingest(&app, &url).await?;
    library_changed(&app);
    Ok(added)
}

#[tauri::command]
async fn refresh(app: AppHandle) -> R<usize> {
    let feeds = store::feeds(&app.state::<Core>().db.lock().unwrap()).map_err(err)?;
    let a = app.clone();
    let ok = bounded(feeds, 6, move |(_, url)| {
        let a = a.clone();
        async move { ingest(&a, &url).await.is_ok() }
    })
    .await
    .into_iter()
    .filter(|ok| *ok)
    .count();
    library_changed(&app);
    // Feed and folder shows of yours are checked with the podcasts.
    sources::check(&app).await;
    Ok(ok)
}

#[derive(Serialize)]
struct Imported {
    added: usize,
    had: usize,
    spotify_only: usize,
    failed: usize,
    /// The shows just added, so Following can point them out.
    ids: Vec<i64>,
    /// Names (or addresses) of the shows that couldn't be reached.
    unreached: Vec<String>,
}

#[tauri::command]
async fn import_opml(app: AppHandle, text: String) -> R<Imported> {
    import_feeds(&app, feed::opml_feeds(&text)?).await
}

/// Everything the Apple Podcasts app follows on this Mac.
#[tauri::command]
async fn import_apple(app: AppHandle) -> R<Imported> {
    let scratch = app.path().app_cache_dir().map_err(err)?;
    let feeds = tauri::async_runtime::spawn_blocking(move || feed::apple_library(&scratch)).await.map_err(err)??;
    if feeds.is_empty() {
        return Err("Apple Podcasts isn't following any shows on this Mac.".into());
    }
    import_feeds(&app, feeds).await
}

/// Follow each feed not already followed. Each comes with the show's name when the source knows it.
async fn import_feeds(app: &AppHandle, feeds: Vec<(String, Option<String>)>) -> R<Imported> {
    let core = app.state::<Core>();
    let known: Vec<String> = store::feeds(&core.db.lock().unwrap())
        .map_err(err)?
        .into_iter()
        .map(|f| f.1)
        .collect();
    let (had, new): (Vec<_>, Vec<_>) = feeds.into_iter().partition(|(u, _)| known.contains(u));
    let a = app.clone();
    let res = bounded(new, 6, move |(u, name): (String, Option<String>)| {
        let a = a.clone();
        async move { ingest(&a, &u).await.map(|(id, _)| id).map_err(|_| name.unwrap_or(u)) }
    })
    .await;
    library_changed(app);
    let (ids, unreached): (Vec<_>, Vec<_>) = res.into_iter().partition(Result::is_ok);
    let ids: Vec<i64> = ids.into_iter().flatten().collect();
    let unreached: Vec<String> = unreached.into_iter().filter_map(Result::err).collect();
    Ok(Imported { added: ids.len(), had: had.len(), spotify_only: 0, failed: unreached.len(), ids, unreached })
}

#[tauri::command]
async fn import_spotify(app: AppHandle, text: String) -> R<Imported> {
    let shows = feed::spotify_shows(&text)?;
    let core = app.state::<Core>();
    let (had, new): (Vec<_>, Vec<_>) = {
        let db = core.db.lock().unwrap();
        shows.into_iter().partition(|(n, _)| store::has_title(&db, n).unwrap_or(false))
    };
    let a = app.clone();
    // Apple's search allows about 20 requests a minute; 3 at a time stays well inside it for a library.
    let res = bounded(new, 3, move |(name, publisher)| {
        let a = a.clone();
        async move {
            match feed::find_feed(&name, &publisher).await {
                Some(url) => ingest(&a, &url).await.map(|(id, _)| id).map_err(|_| (name, publisher)),
                None => Err((name, publisher)),
            }
        }
    })
    .await;
    let mut imp = Imported { added: 0, had: had.len(), spotify_only: 0, failed: 0, ids: vec![], unreached: vec![] };
    {
        let db = core.db.lock().unwrap();
        for r in res {
            match r {
                Ok(id) => {
                    imp.added += 1;
                    imp.ids.push(id);
                }
                Err((n, p)) => {
                    store::add_spotify_only(&db, &n, &p).map_err(err)?;
                    imp.spotify_only += 1;
                }
            }
        }
    }
    library_changed(&app);
    Ok(imp)
}

/// Webview diagnostics into the app's stderr.
#[tauri::command]
fn log(msg: String) {
    eprintln!("[web] {msg}");
}

#[tauri::command]
fn unfollow(app: AppHandle, core: State<Core>, show_id: i64) -> R<()> {
    // The episode playing now keeps playing; its show just leaves the library.
    let files = store::unfollow(&core.db.lock().unwrap(), show_id).map_err(err)?;
    for f in files {
        let _ = std::fs::remove_file(f);
    }
    if let Ok(dir) = app.path().app_data_dir() {
        let _ = std::fs::remove_file(dir.join("art").join(show_id.to_string()));
    }
    library_changed(&app);
    Ok(())
}

/// Point a show at a feed address the person supplies: a Spotify-only show, or a wrong match.
#[tauri::command]
async fn correct_feed(app: AppHandle, show_id: i64, url: String) -> R<String> {
    let (id, title) = ingest(&app, url.trim()).await?;
    if id != show_id {
        let core = app.state::<Core>();
        let _ = store::unfollow(&core.db.lock().unwrap(), show_id);
    }
    library_changed(&app);
    Ok(title)
}

#[tauri::command]
fn episode_notes(core: State<Core>, id: i64) -> R<Option<String>> {
    store::description(&core.db.lock().unwrap(), id).map_err(err)
}

#[tauri::command]
fn shows(core: State<Core>) -> R<Vec<store::ShowRow>> {
    store::shows(&core.db.lock().unwrap()).map_err(err)
}

#[tauri::command]
fn newest(core: State<Core>) -> R<Vec<store::EpisodeRow>> {
    store::newest(&core.db.lock().unwrap(), 40).map_err(err)
}

#[tauri::command]
fn show_episodes(core: State<Core>, show_id: i64) -> R<Vec<store::EpisodeRow>> {
    store::show_episodes(&core.db.lock().unwrap(), show_id).map_err(err)
}

#[tauri::command]
fn settings(core: State<Core>) -> R<HashMap<String, String>> {
    let db = core.db.lock().unwrap();
    let mut st = db.prepare("select key, value from settings").map_err(err)?;
    let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

#[tauri::command]
fn set_setting(app: AppHandle, core: State<Core>, key: String, value: String) -> R<()> {
    store::set_setting(&core.db.lock().unwrap(), &key, &value).map_err(err)?;
    let _ = app.emit("settings", (key, value));
    Ok(())
}

/// After any change to an episode's notes: rewrite its file when there's a notes folder, and tell every window.
fn notes_changed(app: &AppHandle, db: &rusqlite::Connection, episode: i64) {
    if let Some(dir) = marks::folder(db) {
        if let Err(e) = marks::write(db, &dir, episode) {
            eprintln!("[notes] {e}");
        }
    }
    let _ = app.emit("marks", episode);
}

#[tauri::command]
fn add_mark(app: AppHandle, core: State<Core>, episode_id: i64, start: f64, end: f64, quote: Option<String>, chapter: Option<String>) -> R<store::MarkRow> {
    let db = core.db.lock().unwrap();
    let m = store::add_mark(&db, episode_id, start, end, quote.as_deref(), chapter.as_deref()).map_err(err)?;
    notes_changed(&app, &db, episode_id);
    Ok(m)
}

#[tauri::command]
fn set_mark_note(app: AppHandle, core: State<Core>, id: i64, note: String) -> R<()> {
    let db = core.db.lock().unwrap();
    let Some(episode) = store::mark_episode(&db, id).map_err(err)? else { return Ok(()) };
    store::set_mark_note(&db, id, note.trim()).map_err(err)?;
    notes_changed(&app, &db, episode);
    Ok(())
}

#[tauri::command]
fn remove_mark(app: AppHandle, core: State<Core>, id: i64) -> R<()> {
    let db = core.db.lock().unwrap();
    let Some(episode) = store::mark_episode(&db, id).map_err(err)? else { return Ok(()) };
    store::remove_mark(&db, id).map_err(err)?;
    notes_changed(&app, &db, episode);
    Ok(())
}

#[tauri::command]
fn marks(core: State<Core>, episode_id: i64) -> R<Vec<store::MarkRow>> {
    store::marks(&core.db.lock().unwrap(), episode_id).map_err(err)
}

#[tauri::command]
fn all_marks(core: State<Core>) -> R<Vec<store::MarkRow>> {
    store::all_marks(&core.db.lock().unwrap()).map_err(err)
}

/// Where this episode's notes are written, when there's a notes folder.
#[tauri::command]
fn notes_file(core: State<Core>, episode_id: i64) -> R<Option<String>> {
    let db = core.db.lock().unwrap();
    let (Some(dir), Some(e)) = (marks::folder(&db), store::episode(&db, episode_id).map_err(err)?) else { return Ok(None) };
    Ok(Some(marks::file_for(&dir, &e).to_string_lossy().into_owned()))
}

/// Choose the notes folder (or none). Choosing one writes every episode that already has notes; returns how many.
#[tauri::command]
fn set_notes_folder(app: AppHandle, core: State<Core>, path: Option<String>) -> R<usize> {
    let db = core.db.lock().unwrap();
    let value = path.unwrap_or_default();
    store::set_setting(&db, "notes_folder", &value).map_err(err)?;
    let _ = app.emit("settings", ("notes_folder", &value));
    Ok(marks::folder(&db).map(|dir| marks::write_all(&db, &dir)).unwrap_or(0))
}

/// Play the linked episode from the note's time, bring the main window forward and show the episode's notes.
fn open_note_link(app: &AppHandle, url: &str) {
    let Some((id, t)) = marks::parse_link(url) else { return };
    if let Err(e) = play::choose(app.clone(), app.state::<Core>(), id, None, Some(t)) {
        eprintln!("[notes] {url}: {e}");
        return;
    }
    #[cfg(target_os = "macos")]
    let _ = modes::set_mode(app.clone(), "win".into());
    let _ = app.emit("show-notes", id);
}

// ---------- Read to me ----------

#[derive(Serialize)]
struct ServiceInfo {
    id: &'static str,
    name: &'static str,
    model: &'static str,
    voice: &'static str,
    /// A key is saved in the Keychain (the key itself never comes back to the page).
    keyed: bool,
}

#[tauri::command]
fn voice_services(core: State<Core>) -> Vec<ServiceInfo> {
    let saved = voice::keys_saved(&core.db.lock().unwrap());
    voice::SERVICES
        .iter()
        .map(|s| ServiceInfo { id: s.id, name: s.name, model: s.model, voice: s.voice, keyed: saved.get(s.id).copied().unwrap_or(false) })
        .collect()
}

#[tauri::command]
fn set_voice_key(core: State<Core>, service: String, key: String) -> R<()> {
    voice::service(&service).ok_or("That service isn't known.")?;
    voice::set_key(&service, &key)?;
    store::set_setting(&core.db.lock().unwrap(), &format!("voice_{service}_keyed"), if key.trim().is_empty() { "" } else { "1" }).map_err(err)
}

#[tauri::command]
async fn mac_voices() -> Vec<voice::MacVoice> {
    tauri::async_runtime::spawn_blocking(voice::mac_voices).await.unwrap_or_default()
}

/// One sentence in the current voice (or a show's own), as a data address the page can play.
#[tauri::command]
async fn voice_sample(app: AppHandle, show_id: Option<i64>) -> R<String> {
    let v = {
        let core = app.state::<Core>();
        let db = core.db.lock().unwrap();
        let own = show_id.and_then(|id| store::show_voice(&db, id));
        voice::resolve(&db, own.as_deref())
    };
    let a = voice::speak(&v, voice::SAMPLE).await?;
    use base64::Engine;
    let mime = if a.ext == "wav" { "audio/wav" } else { "audio/mpeg" };
    Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(&a.bytes)))
}

#[derive(Serialize)]
struct Page {
    /// Where the address ended up after redirects.
    url: String,
    /// "page" (an article to read), "podcast" (a feed with audio) or "feed" (a feed of articles)
    kind: &'static str,
    html: String,
    title: Option<String>,
}

/// Fetch an address the person gave and say what it is. The page reads articles itself (Readability).
#[tauri::command]
async fn fetch_page(url: String) -> R<Page> {
    let res = feed::HTTP.get(url.trim()).send().await.map_err(|_| format!("Couldn't reach {}.", url.trim()))?;
    if !res.status().is_success() {
        return Err(format!("That page answered with an error ({}).", res.status().as_u16()));
    }
    let final_url = res.url().to_string();
    let ctype = res.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_lowercase();
    if ctype.starts_with("audio/") || ctype.starts_with("video/") || ctype.starts_with("image/") {
        return Err("That's not a page with words to read.".into());
    }
    let body = res.bytes().await.map_err(err)?;
    let html = String::from_utf8_lossy(&body[..body.len().min(8 * 1024 * 1024)]).to_string();
    let head = html.trim_start().get(..400).unwrap_or(html.trim_start()).to_lowercase();
    let is_feed = ctype.contains("rss") || ctype.contains("atom") || (ctype.contains("xml") && !ctype.contains("html")) || head.starts_with("<?xml") && (head.contains("<rss") || head.contains("<feed")) || head.starts_with("<rss") || head.starts_with("<feed");
    let title = |s: &str| s.split_once("<title").and_then(|(_, r)| r.split_once('>')).and_then(|(_, r)| r.split_once("</title>")).map(|(t, _)| t.replace("<![CDATA[", "").replace("]]>", "").trim().to_string());
    let kind = if !is_feed {
        "page"
    } else if html.contains("<enclosure") && (html.contains("type=\"audio") || html.contains(".mp3") || html.contains(".m4a")) {
        "podcast"
    } else {
        "feed"
    };
    Ok(Page { url: final_url, kind, title: if kind == "page" { None } else { title(&html) }, html })
}

#[tauri::command]
async fn read_file(path: String) -> R<read::Doc> {
    tauri::async_runtime::spawn_blocking(move || read::read_file(std::path::Path::new(&path))).await.map_err(err)?
}

/// The Read to me show's id, making it the first time.
#[tauri::command]
fn read_to_me(core: State<Core>) -> R<i64> {
    store::read_to_me(&core.db.lock().unwrap()).map_err(err)
}

/// Queue something to be read into a show of yours: one you name (`new_show`), the one given, or Read to me.
#[tauri::command]
fn add_read(app: AppHandle, core: State<Core>, show_id: Option<i64>, new_show: Option<String>, title: String, source: String, text: String) -> R<i64> {
    let text = text.trim();
    if text.is_empty() {
        return Err("There's nothing to read in this.".into());
    }
    let id = {
        let db = core.db.lock().unwrap();
        let show = match (new_show.as_deref().map(str::trim).filter(|n| !n.is_empty()), show_id) {
            (Some(name), _) => store::make_show(&db, name, "items", None, false).map_err(err)?,
            (None, Some(id)) => id,
            (None, None) => store::read_to_me(&db).map_err(err)?,
        };
        let title = if title.trim().is_empty() { "Untitled" } else { title.trim() };
        let id = store::add_read(&db, show, title, &source, None, text.chars().count() as i64, "queued").map_err(err)?.ok_or("That's already here.")?;
        let d = read::dir(&app);
        std::fs::create_dir_all(&d).map_err(err)?;
        std::fs::write(d.join(format!("{id}.txt")), text).map_err(err)?;
        id
    };
    read::wake();
    let _ = app.emit("reads", ());
    library_changed(&app);
    Ok(id)
}

/// A new, empty show of yours, to move things into.
#[tauri::command]
fn make_empty_show(app: AppHandle, core: State<Core>, name: String) -> R<i64> {
    let name = name.trim();
    let id = store::make_show(&core.db.lock().unwrap(), if name.is_empty() { "New show" } else { name }, "items", None, false).map_err(err)?;
    library_changed(&app);
    Ok(id)
}

#[tauri::command]
fn move_episode(app: AppHandle, core: State<Core>, id: i64, show_id: i64) -> R<()> {
    if !store::move_episode(&core.db.lock().unwrap(), id, show_id).map_err(err)? {
        return Err("That episode can only stay where it is.".into());
    }
    library_changed(&app);
    Ok(())
}

/// One read, finished or not: the pane follows the one you just asked for until it's an episode.
#[tauri::command]
fn read_status(core: State<Core>, id: i64) -> R<Option<store::ReadRow>> {
    store::read(&core.db.lock().unwrap(), id).map_err(err)
}

#[tauri::command]
fn reads(core: State<Core>) -> R<Vec<store::ReadRow>> {
    store::pending_reads(&core.db.lock().unwrap()).map_err(err)
}

/// Read: a waiting one joins the queue; Try again carries on from the failed piece.
#[tauri::command]
fn read_now(app: AppHandle, core: State<Core>, id: i64) -> R<()> {
    store::set_read_state(&core.db.lock().unwrap(), id, "queued", None).map_err(err)?;
    read::wake();
    let _ = app.emit("reads", ());
    Ok(())
}

/// Stop: back to waiting. Finished pieces are kept for when it's read again.
#[tauri::command]
fn stop_read(app: AppHandle, core: State<Core>, id: i64) -> R<()> {
    store::set_read_state(&core.db.lock().unwrap(), id, "waiting", None).map_err(err)?;
    let _ = app.emit("reads", ());
    Ok(())
}

/// Remove something that hasn't become an episode yet.
#[tauri::command]
fn discard_read(app: AppHandle, core: State<Core>, id: i64) -> R<()> {
    store::remove_read(&core.db.lock().unwrap(), id).map_err(err)?;
    let d = read::dir(&app);
    let _ = std::fs::remove_dir_all(d.join(id.to_string()));
    let _ = std::fs::remove_file(d.join(format!("{id}.txt")));
    let _ = app.emit("reads", ());
    library_changed(&app);
    Ok(())
}

#[tauri::command]
fn set_show(app: AppHandle, core: State<Core>, id: i64, title: Option<String>, auto: Option<bool>, voice: Option<String>) -> R<()> {
    let db = core.db.lock().unwrap();
    store::set_show(&db, id, title.as_deref().map(str::trim).filter(|t| !t.is_empty()), auto, voice.as_deref()).map_err(err)?;
    // Switching to automatic reads what's waiting.
    if auto == Some(true) && store::queue_waiting(&db, id).map_err(err)? > 0 {
        read::wake();
        let _ = app.emit("reads", ());
    }
    drop(db);
    library_changed(&app);
    Ok(())
}

/// What a feed or folder holds, for the card: its name, its newest titles, and how many.
#[tauri::command]
async fn preview_source(kind: String, source: String) -> R<sources::Preview> {
    sources::preview(&kind, source.trim()).await
}

#[tauri::command]
async fn make_source_show(app: AppHandle, kind: String, source: String, name: String, auto: bool) -> R<i64> {
    if kind != "feed" && kind != "folder" {
        return Err("Only a feed or a folder can become a show.".into());
    }
    let name = if name.trim().is_empty() { "New show".to_string() } else { name.trim().to_string() };
    let source = source.trim();
    let source = if kind == "folder" && source.len() > 1 { source.trim_end_matches('/') } else { source };
    sources::make(&app, &kind, source, &name, auto).await
}

/// A summary-only post's page, read by the window (Readability); none when only the preview could be had.
#[tauri::command]
fn set_read_text(app: AppHandle, core: State<Core>, id: i64, text: Option<String>, preview: bool) -> R<()> {
    let chars = match text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => {
            std::fs::write(read::dir(&app).join(format!("{id}.txt")), t).map_err(err)?;
            Some(t.chars().count() as i64)
        }
        None => None,
    };
    store::set_read_text(&core.db.lock().unwrap(), id, chars, preview).map_err(err)?;
    read::wake();
    let _ = app.emit("reads", ());
    Ok(())
}

#[tauri::command]
fn is_folder(path: String) -> bool {
    std::path::Path::new(&path).is_dir()
}

fn forget_episode_files(app: &AppHandle, episode: i64) {
    if let Ok(data) = app.path().app_data_dir() {
        for kind in ["transcripts", "chapters"] {
            let _ = std::fs::remove_file(data.join(kind).join(format!("{episode}.txt")));
        }
    }
}

/// Remove a show of yours, its episodes and everything made for them.
#[tauri::command]
fn remove_show(app: AppHandle, core: State<Core>, id: i64) -> R<()> {
    let (episodes, reads) = {
        let db = core.db.lock().unwrap();
        (store::show_episode_ids(&db, id).map_err(err)?, store::show_read_ids(&db, id).map_err(err)?)
    };
    for e in episodes {
        play::forget(&app, e);
        forget_episode_files(&app, e);
    }
    let d = read::dir(&app);
    for r in reads {
        let _ = std::fs::remove_dir_all(d.join(r.to_string()));
        let _ = std::fs::remove_file(d.join(format!("{r}.txt")));
    }
    let _ = app.emit("reads", ());
    unfollow(app, core, id)
}

/// Delete one episode of yours: its audio, transcript and chapters go with it.
#[tauri::command]
fn delete_episode(app: AppHandle, core: State<Core>, id: i64) -> R<()> {
    play::forget(&app, id);
    let path = store::delete_episode(&core.db.lock().unwrap(), id).map_err(err)?;
    if let Some(p) = path {
        let _ = std::fs::remove_file(p);
    }
    forget_episode_files(&app, id);
    library_changed(&app);
    Ok(())
}

/// What arrived from outside (Open With, the Dock, zenpod://read) since the page last asked.
#[tauri::command]
fn take_incoming(core: State<Core>) -> Vec<Incoming> {
    std::mem::take(&mut *core.incoming.lock().unwrap())
}

fn hand_in(app: &AppHandle, item: Incoming) {
    app.state::<Core>().incoming.lock().unwrap().push(item);
    #[cfg(target_os = "macos")]
    let _ = modes::set_mode(app.clone(), "win".into());
    let _ = app.emit("incoming", ());
}

/// zenpod://read?url=…&show=…
fn open_read_link(app: &AppHandle, url: &tauri::Url) {
    let q: HashMap<String, String> = url.query_pairs().into_owned().collect();
    let Some(target) = q.get("url").filter(|u| u.starts_with("http://") || u.starts_with("https://")) else { return };
    hand_in(app, Incoming { kind: "link", value: target.clone(), show: q.get("show").cloned() });
}

/// Release builds abort on panic with nothing on screen; keep the message and where it happened.
fn log_panics() {
    let Some(home) = std::env::var_os("HOME") else { return };
    let dir = std::path::Path::new(&home).join("Library/Logs/Zenpod");
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("panic.log")) {
            let thread = std::thread::current();
            let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let _ = writeln!(f, "--- {when} {} {}\n{info}\n{}", env!("CARGO_PKG_VERSION"), thread.name().unwrap_or("?"), std::backtrace::Backtrace::force_capture());
        }
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log_panics();
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init()).plugin(
        tauri_plugin_window_state::Builder::new()
            .with_denylist(&["player", "panel"])
            .with_state_flags(tauri_plugin_window_state::StateFlags::POSITION | tauri_plugin_window_state::StateFlags::SIZE)
            .build(),
    );
    builder
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = store::open(&dir.join("listener.db")).map_err(err)?;
            if let Some(w) = app.get_webview_window("main") {
                // Debug builds: LISTENER_BOUNDS="x,y,w,h" places the window for automated checks.
                #[cfg(debug_assertions)]
                if let Some(b) = std::env::var("LISTENER_BOUNDS").ok().map(|v| v.split(',').filter_map(|n| n.parse::<f64>().ok()).collect::<Vec<_>>()) {
                    if let [x, y, width, height] = b[..] {
                        let _ = w.set_size(tauri::LogicalSize::new(width, height));
                        let _ = w.set_position(tauri::LogicalPosition::new(x, y));
                    }
                }
                let _ = w.set_theme(match store::setting(&db, "look").as_deref() {
                    Some("day") => Some(tauri::Theme::Light),
                    Some("night") => Some(tauri::Theme::Dark),
                    _ => None,
                });
            }
            app.manage(Core {
                db: Mutex::new(db),
                now: Mutex::new(play::Now::default()),
                fetching: Mutex::new(HashSet::new()),
                urls: Mutex::new(HashMap::new()),
                incoming: Mutex::new(vec![]),
            });
            play::restore(app.handle());
            play::tidy(app.handle());
            play::spawn_player(app.handle())?;
            #[cfg(target_os = "macos")]
            modes::spawn(app.handle())?;
            read::spawn(app.handle());

            // A note's time clicked in the notes folder (zenpod://episode/<id>?t=<s>): open the window there.
            // zenpod://read?url=… sends a page to Read to me.
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let h = app.handle().clone();
                app.deep_link().on_open_url(move |e| {
                    for url in e.urls() {
                        if url.host_str() == Some("read") {
                            open_read_link(&h, &url);
                        } else {
                            open_note_link(&h, url.as_str());
                        }
                    }
                });
            }

            // Refresh quietly on launch, then hourly.
            let h = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let _ = refresh(h.clone()).await;
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
            });

            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("listener", |ctx, req, responder| {
            let app = ctx.app_handle().clone();
            // Answer on the main thread: WebKit cancels requests there, so it can't cancel one mid-answer.
            tauri::async_runtime::spawn(async move {
                let res = proto::handle(app.clone(), req).await;
                let _ = app.run_on_main_thread(move || responder.respond(res));
            });
        })
        // Closing the window only hides it; the player window keeps playing.
        .on_window_event(|w, e| match e {
            WindowEvent::CloseRequested { api, .. } if w.label() == "main" => {
                api.prevent_close();
                let _ = w.hide();
            }
            // The Pill's panel travels with the Pill.
            WindowEvent::Moved(_) if w.label() == "pill" => {
                if w.app_handle().get_webview_window("panel").and_then(|p| p.is_visible().ok()).unwrap_or(false) {
                    let _ = modes::pill_panel(w.app_handle().clone(), true);
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            log, add_show, refresh, import_opml, import_spotify, import_apple, shows, newest, show_episodes, settings, set_setting,
            play::playback, play::player_ready, play::choose, play::step, play::follow_list, play::toggle, play::seek, play::skip,
            play::set_speed, play::report, play::peaks, episode_notes, unfollow, correct_feed, play::chapters, play::transcript, play::keep,
            modes::set_mode, modes::pill_panel, modes::drag_panel, proto::warm,
            add_mark, set_mark_note, remove_mark, marks, all_marks, notes_file, set_notes_folder,
            voice_services, set_voice_key, mac_voices, voice_sample, fetch_page, read_file, read_to_me, add_read, reads,
            read_now, stop_read, discard_read, set_show, remove_show, delete_episode, take_incoming,
            preview_source, make_source_show, set_read_text, is_folder, read_status, make_empty_show, move_episode
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, e| match e {
            RunEvent::Reopen { .. } => {
                // The Dock icon brings forward whichever surface is showing; it only opens the window when none is.
                if !modes::raise(app) {
                    let _ = modes::set_mode(app.clone(), "win".into());
                }
            }
            // A file dropped on the Dock icon or opened with Zenpod: Read to me takes it.
            #[cfg(target_os = "macos")]
            RunEvent::Opened { urls } => {
                for u in urls {
                    if let Ok(p) = u.to_file_path() {
                        hand_in(app, Incoming { kind: "file", value: p.to_string_lossy().into_owned(), show: None });
                    } else if u.scheme() == "zenpod" && u.host_str() == Some("read") {
                        open_read_link(app, &u);
                    }
                }
            }
            _ => {}
        });
}
