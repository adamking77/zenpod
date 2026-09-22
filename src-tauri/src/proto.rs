//! `listener://localhost/audio/<episode>` and `listener://localhost/art/<show>`.
//! Everything the webviews load goes through here, so it carries CORS and the analyser can read it.
//! Audio is served from disk when downloaded and proxied with ranges otherwise, so seeking stays instant.

use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use tauri::http::{header, Request, Response, StatusCode};
use tauri::{AppHandle, Manager};

use crate::{feed::HTTP, store, Core};

// WebKit asks for open-ended ranges; answering in slices keeps memory flat and it simply asks again.
const SLICE: u64 = 2 * 1024 * 1024;

type Res = Response<Vec<u8>>;

pub async fn handle(app: AppHandle, req: Request<Vec<u8>>) -> Res {
    let path = req.uri().path().to_string();
    let range = req.headers().get(header::RANGE).and_then(|v| v.to_str().ok()).map(str::to_string);
    let mut parts = path.trim_start_matches('/').split('/');
    let res = match (parts.next(), parts.next().and_then(|id| id.parse::<i64>().ok())) {
        (Some("audio"), Some(id)) => audio(&app, id, range.clone()).await,
        (Some("art"), Some(id)) => art(&app, id).await,
        _ => Err(StatusCode::NOT_FOUND),
    };
    let mut r = res.unwrap_or_else(|code| Response::builder().status(code).body(Vec::new()).unwrap());
    #[cfg(debug_assertions)]
    eprintln!("[proto] {path} {range:?} -> {} {}", r.status(), r.body().len());
    r.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
    r
}

/// "bytes=a-b" → (a, Some(b)); anything else starts at 0.
fn parse_range(h: Option<&str>) -> (u64, Option<u64>) {
    let Some(spec) = h.and_then(|h| h.strip_prefix("bytes=")) else { return (0, None) };
    let (a, b) = spec.split_once('-').unwrap_or((spec, ""));
    (a.trim().parse().unwrap_or(0), b.trim().parse().ok())
}

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().map(str::to_lowercase).as_deref() {
        Some("m4a" | "mp4" | "aac") => "audio/mp4",
        Some("wav") => "audio/wav",
        Some("ogg" | "oga" | "opus") => "audio/ogg",
        _ => "audio/mpeg",
    }
}

fn partial(body: Vec<u8>, start: u64, total: u64, ctype: &str) -> Res {
    let end = start + body.len() as u64 - 1;
    Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, ctype)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{total}"))
        .body(body)
        .unwrap()
}

async fn audio(app: &AppHandle, id: i64, range: Option<String>) -> Result<Res, StatusCode> {
    let (local, remote) = {
        let core = app.state::<Core>();
        let db = core.db.lock().unwrap();
        store::audio_source(&db, id).map_err(|_| StatusCode::NOT_FOUND)?
    };
    let (start, end) = parse_range(range.as_deref());

    let dir = app.path().app_data_dir().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.join("audio");
    let part = dir.join(format!("{id}.part"));
    // Downloaded, or downloading far enough: read from disk.
    let on_disk = match local.filter(|p| std::path::Path::new(p).exists()) {
        Some(p) => Some((PathBuf::from(p), None)),
        None => std::fs::metadata(&part).ok().filter(|m| m.len() > start + SLICE.min(64 * 1024)).map(|m| (part, Some(m.len()))),
    };
    if let Some((path, growing)) = on_disk {
        let total = match growing {
            Some(_) => remote_total(app, id, &remote).await,
            None => None,
        };
        return tauri::async_runtime::spawn_blocking(move || -> std::io::Result<Res> {
            let mut f = std::fs::File::open(&path)?;
            let have = f.metadata()?.len();
            let total = total.unwrap_or(have);
            if have == 0 || start >= have {
                return Ok(Response::builder().status(StatusCode::RANGE_NOT_SATISFIABLE).body(Vec::new()).unwrap());
            }
            let last = end.unwrap_or(u64::MAX).min(have - 1).min(start + SLICE - 1);
            let mut buf = vec![0; (last - start + 1) as usize];
            f.seek(SeekFrom::Start(start))?;
            f.read_exact(&mut buf)?;
            Ok(partial(buf, start, total, mime(&path.to_string_lossy())))
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Streaming: whole aligned blocks from the host, kept in memory, sliced to what WebKit asked for.
    let (block, total, ctype) = fetch_block(app, id, &remote, start / BLOCK).await?;
    let off = (start % BLOCK) as usize;
    if off >= block.len() {
        return Err(StatusCode::RANGE_NOT_SATISFIABLE);
    }
    let last = end.map(|e| (e - start) as usize).unwrap_or(usize::MAX).min(block.len() - off - 1);
    Ok(partial(block[off..=off + last].to_vec(), start, total, &ctype))
}

const BLOCK: u64 = 1024 * 1024;
type Block = (std::sync::Arc<Vec<u8>>, u64, String);

static BLOCKS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<(i64, u64), Block>>> =
    std::sync::LazyLock::new(Default::default);

async fn remote_total(app: &AppHandle, id: i64, remote: &str) -> Option<u64> {
    fetch_block(app, id, remote, 0).await.ok().map(|b| b.1)
}

async fn fetch_block(app: &AppHandle, id: i64, remote: &str, n: u64) -> Result<Block, StatusCode> {
    if let Some(b) = BLOCKS.lock().unwrap().get(&(id, n)) {
        return Ok(b.clone());
    }
    let url = crate::resolved_url(app, id, remote).await;
    let r = HTTP
        .get(&url)
        .header(header::RANGE, format!("bytes={}-{}", n * BLOCK, (n + 1) * BLOCK - 1))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let ctype = r.headers().get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("audio/mpeg").to_string();
    let total = r
        .headers()
        .get(header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit('/').next())
        .and_then(|t| t.parse::<u64>().ok());
    let partial = r.status() == StatusCode::PARTIAL_CONTENT;
    let body = r.bytes().await.map_err(|_| StatusCode::BAD_GATEWAY)?.to_vec();
    let (body, total) = match (partial, total) {
        (true, Some(t)) => (body, t),
        // ponytail: a host that ignores ranges sends the whole file on every block; fine until one shows up.
        _ => {
            let t = body.len() as u64;
            let a = (n * BLOCK).min(t) as usize;
            (body[a..((n + 1) * BLOCK).min(t) as usize].to_vec(), t)
        }
    };
    let b = (std::sync::Arc::new(body), total, ctype);
    let mut cache = BLOCKS.lock().unwrap();
    if cache.len() >= 24 {
        cache.clear(); // ponytail: drop-all eviction, 24 MB ceiling; LRU if seeking ever thrashes
    }
    cache.insert((id, n), b.clone());
    Ok(b)
}

fn sniff(b: &[u8]) -> &'static str {
    match b {
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "image/webp",
        [b'G', b'I', b'F', ..] => "image/gif",
        _ => "image/jpeg",
    }
}

async fn art(app: &AppHandle, show: i64) -> Result<Res, StatusCode> {
    let dir: PathBuf = app.path().app_data_dir().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.join("art");
    let file = dir.join(show.to_string());
    let bytes = match std::fs::read(&file) {
        Ok(b) => b,
        Err(_) => {
            let url = {
                let core = app.state::<Core>();
                let db = core.db.lock().unwrap();
                store::show_image(&db, show).ok().flatten().ok_or(StatusCode::NOT_FOUND)?
            };
            let b = HTTP.get(&url).send().await.and_then(|r| r.error_for_status()).map_err(|_| StatusCode::BAD_GATEWAY)?;
            let b = b.bytes().await.map_err(|_| StatusCode::BAD_GATEWAY)?.to_vec();
            let _ = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, &b));
            b
        }
    };
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, sniff(&bytes))
        .header(header::CACHE_CONTROL, "max-age=86400")
        .body(bytes)
        .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(parse_range(None), (0, None));
        assert_eq!(parse_range(Some("bytes=0-1")), (0, Some(1)));
        assert_eq!(parse_range(Some("bytes=500-")), (500, None));
        assert_eq!(mime("/x/12.m4a"), "audio/mp4");
        assert_eq!(mime("/x/12.mp3"), "audio/mpeg");
    }
}
