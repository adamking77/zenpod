mod feed;
#[cfg(target_os = "macos")]
mod panel_test;
mod peaks;
mod play;
mod proto;
mod store;

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
}

/// Follow an enclosure's redirect chain once per session; range requests then go straight to the host.
pub async fn resolved_url(app: &AppHandle, id: i64, remote: &str) -> String {
    if let Some(u) = app.state::<Core>().urls.lock().unwrap().get(&id).cloned() {
        return u;
    }
    let u = feed::HTTP
        .get(remote)
        .header("range", "bytes=0-0")
        .send()
        .await
        .map(|r| r.url().to_string())
        .unwrap_or_else(|_| remote.to_string());
    app.state::<Core>().urls.lock().unwrap().insert(id, u.clone());
    u
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
async fn add_show(app: AppHandle, input: String) -> R<String> {
    let input = input.trim();
    let url = if input.starts_with("http://") || input.starts_with("https://") {
        input.to_string()
    } else {
        feed::find_feed(input, "")
            .await
            .ok_or_else(|| format!("No public feed was found for “{input}”."))?
    };
    let (_, title) = ingest(&app, &url).await?;
    library_changed(&app);
    Ok(title)
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
    Ok(ok)
}

#[derive(Serialize)]
struct Imported {
    added: usize,
    had: usize,
    spotify_only: usize,
    failed: usize,
}

#[tauri::command]
async fn import_opml(app: AppHandle, text: String) -> R<Imported> {
    let urls = feed::opml_feeds(&text)?;
    let core = app.state::<Core>();
    let known: Vec<String> = store::feeds(&core.db.lock().unwrap())
        .map_err(err)?
        .into_iter()
        .map(|f| f.1)
        .collect();
    let (had, new): (Vec<_>, Vec<_>) = urls.into_iter().partition(|u| known.contains(u));
    let a = app.clone();
    let res = bounded(new, 6, move |u| {
        let a = a.clone();
        async move { ingest(&a, &u).await.is_ok() }
    })
    .await;
    library_changed(&app);
    let added = res.iter().filter(|ok| **ok).count();
    Ok(Imported { added, had: had.len(), spotify_only: 0, failed: res.len() - added })
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
                Some(url) => ingest(&a, &url).await.map(|_| true).map_err(|_| (name, publisher)),
                None => Err((name, publisher)),
            }
        }
    })
    .await;
    let mut imp = Imported { added: 0, had: had.len(), spotify_only: 0, failed: 0 };
    {
        let db = core.db.lock().unwrap();
        for r in res {
            match r {
                Ok(_) => imp.added += 1,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
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
            });
            play::restore(app.handle());
            play::spawn_player(app.handle())?;

            // Refresh quietly on launch, then hourly.
            let h = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let _ = refresh(h.clone()).await;
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
            });

            #[cfg(target_os = "macos")]
            if let Ok(v) = std::env::var("LISTENER_PANEL_TEST") {
                panel_test::spawn(app.handle(), &v)?;
            }
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("listener", |ctx, req, responder| {
            let app = ctx.app_handle().clone();
            tauri::async_runtime::spawn(async move { responder.respond(proto::handle(app, req).await) });
        })
        // Closing the window only hides it; the player window keeps playing.
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                if w.label() == "main" {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            log, add_show, refresh, import_opml, import_spotify, shows, newest, show_episodes, settings, set_setting,
            play::playback, play::player_ready, play::choose, play::toggle, play::seek, play::skip,
            play::set_speed, play::report, play::peaks, episode_notes, unfollow, correct_feed
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, e| {
            if let RunEvent::Reopen { .. } = e {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        });
}
