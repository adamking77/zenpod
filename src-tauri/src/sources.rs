//! Shows that keep getting new episodes: a blog's feed or a folder. Checked whenever podcasts refresh. Each new post
//! or file becomes a read, waiting for you or queued at once, as the show says. Everything already there when the show
//! was made is remembered as seen, apart from the 3 newest, which arrive as its first episodes.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{feed::HTTP, read, store, Core};

/// The 3 newest arrive when a show is made; older ones are left alone.
const FIRST: usize = 3;
/// A feed whose post is shorter than this carries a summary; the page is fetched for the whole post.
const WHOLE: usize = 1500;

#[derive(Debug, Clone)]
pub struct Item {
    pub key: String,
    pub title: String,
    pub link: Option<String>,
    /// The post as the feed carries it, as text.
    pub text: String,
}

#[derive(Serialize)]
pub struct Preview {
    pub name: String,
    pub items: Vec<String>,
    pub count: usize,
}

// ---------- feeds ----------

pub async fn fetch_feed(url: &str) -> Result<(String, Vec<Item>), String> {
    let body = HTTP
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|_| format!("Couldn't reach {url}."))?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    parse_feed(&body)
}

fn kid<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

/// RSS or Atom, newest first as the feed lists them.
pub fn parse_feed(xml: &str) -> Result<(String, Vec<Item>), String> {
    let doc = roxmltree::Document::parse(xml).map_err(|_| "That address isn't a feed.".to_string())?;
    let root = doc.root_element();
    let text_of = |n: Option<roxmltree::Node>| n.map(|n| n.text().unwrap_or("").trim().to_string()).unwrap_or_default();
    let (title, entries): (String, Vec<roxmltree::Node>) = match root.tag_name().name() {
        "rss" | "RDF" => {
            let ch = kid(root, "channel").ok_or("That address isn't a feed.")?;
            let items = if root.tag_name().name() == "RDF" { root.children().filter(|c| c.tag_name().name() == "item").collect() } else { ch.children().filter(|c| c.tag_name().name() == "item").collect() };
            (text_of(kid(ch, "title")), items)
        }
        "feed" => (text_of(kid(root, "title")), root.children().filter(|c| c.tag_name().name() == "entry").collect()),
        _ => return Err("That address isn't a feed.".into()),
    };
    let items = entries
        .into_iter()
        .filter_map(|e| {
            let title = html_text(&text_of(kid(e, "title"))).replace("\n\n", " ");
            let link = kid(e, "link").and_then(|l| l.attribute("href").map(str::to_string).or_else(|| l.text().map(|t| t.trim().to_string()))).filter(|l| !l.is_empty());
            // Atom may list several links; the page is the alternate one.
            let link = e
                .children()
                .find(|c| c.tag_name().name() == "link" && c.attribute("rel").is_none_or(|r| r == "alternate") && c.attribute("href").is_some())
                .and_then(|l| l.attribute("href").map(str::to_string))
                .or(link);
            let key = [kid(e, "guid"), kid(e, "id")].into_iter().flatten().map(|n| n.text().unwrap_or("").trim().to_string()).find(|k| !k.is_empty()).or_else(|| link.clone()).unwrap_or_else(|| title.clone());
            // The whole post when the feed carries it (content:encoded, Atom content), else the summary.
            let body = e
                .children()
                .filter(|c| c.is_element() && matches!(c.tag_name().name(), "encoded" | "content" | "description" | "summary"))
                .map(|c| c.text().unwrap_or("").to_string())
                .max_by_key(String::len)
                .unwrap_or_default();
            (!title.is_empty() || !body.is_empty()).then(|| Item { key, title: if title.is_empty() { "Untitled".into() } else { title }, link, text: html_text(&body) })
        })
        .collect();
    Ok((if title.is_empty() { "Feed".into() } else { html_text(&title).replace("\n\n", " ") }, items))
}

/// A post's HTML as paragraphs and "# " headings; code, tables, figures, scripts and styles are left out.
pub fn html_text(html: &str) -> String {
    let mut out = String::new();
    let mut skip = 0usize;
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        if skip == 0 {
            out.push_str(&entities(&rest[..lt]));
        }
        let Some(gt) = rest[lt..].find('>') else { break };
        let tag = rest[lt + 1..lt + gt].trim().to_lowercase();
        rest = &rest[lt + gt + 1..];
        let closing = tag.starts_with('/');
        let name: String = tag.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        if matches!(name.as_str(), "script" | "style" | "pre" | "code" | "table" | "figure" | "svg" | "noscript" | "iframe") && !tag.ends_with('/') {
            if closing { skip = skip.saturating_sub(1) } else { skip += 1 }
            continue;
        }
        if skip > 0 {
            continue;
        }
        match name.as_str() {
            "p" | "div" | "li" | "blockquote" | "section" | "article" | "ul" | "ol" | "br" | "hr" | "dd" | "dt" => out.push_str("\n\n"),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => out.push_str(if closing { "\n\n" } else { "\n\n# " }),
            _ => {}
        }
    }
    if skip == 0 {
        out.push_str(&entities(rest));
    }
    out.split("\n\n")
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty() && p != "#")
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let end = tail.find(';').filter(|e| *e <= 10);
        let Some(end) = end else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let name = &tail[1..end];
        let ch = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some(' '),
            "rsquo" => Some('’'),
            "lsquo" => Some('‘'),
            "rdquo" => Some('”'),
            "ldquo" => Some('“'),
            "mdash" => Some('—'),
            "ndash" => Some('–'),
            "hellip" => Some('…'),
            n if n.starts_with("#x") || n.starts_with("#X") => u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32),
            n if n.starts_with('#') => n[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &tail[end + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

// ---------- folders ----------

const READABLE: &[&str] = &["md", "markdown", "txt", "pdf", "docx", "doc", "rtf", "html", "htm"];

/// A file changed this recently may still be being written; it's picked up on the next check instead.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(30);

/// Readable files directly in the folder, newest first, leaving out any still being written.
pub fn folder_files(dir: &Path) -> Result<Vec<(PathBuf, std::time::SystemTime)>, String> {
    let settled = std::time::SystemTime::now() - SETTLE;
    let rd = std::fs::read_dir(dir).map_err(|_| "That folder isn't there any more.".to_string())?;
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')))
        .filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| READABLE.contains(&e.to_lowercase().as_str())))
        .filter_map(|p| Some((p.clone(), p.metadata().ok()?.modified().ok()?)))
        .filter(|(_, m)| *m <= settled)
        .collect();
    files.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(files)
}

fn file_title(p: &Path) -> String {
    p.file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").replace(['-', '_'], " ")
}

pub async fn preview(kind: &str, source: &str) -> Result<Preview, String> {
    match kind {
        "feed" => {
            let (name, items) = fetch_feed(source).await?;
            Ok(Preview { name, count: items.len(), items: items.into_iter().take(FIRST).map(|i| i.title).collect() })
        }
        _ => {
            let dir = PathBuf::from(source);
            let files = folder_files(&dir)?;
            let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("Folder").replace(['-', '_'], " ");
            Ok(Preview { name, count: files.len(), items: files.iter().take(FIRST).map(|(p, _)| p.file_name().unwrap_or_default().to_string_lossy().into_owned()).collect() })
        }
    }
}

// ---------- making and checking ----------

/// A new read from a source. Text the feed carried is stored now; a summary-only post waits for its page.
fn add(app: &AppHandle, db: &rusqlite::Connection, show: i64, auto: bool, it: &Item, fallback: &str) -> Result<(), String> {
    let (text, link) = (&it.text, it.link.as_deref());
    let needs = link.is_some() && text.chars().count() < WHOLE;
    let state = if auto { "queued" } else { "waiting" };
    let Some(id) = store::add_source_read(db, show, &it.title, link.unwrap_or(fallback), &it.key, text.chars().count() as i64, state, link, needs).map_err(|e| e.to_string())? else { return Ok(()) };
    let d = read::dir(app);
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    std::fs::write(d.join(format!("{id}.txt")), text).map_err(|e| e.to_string())
}

/// Make a show from a feed or a folder: everything there now is seen, the 3 newest arrive.
pub async fn make(app: &AppHandle, kind: &str, source: &str, name: &str, auto: bool) -> Result<i64, String> {
    let keys = listing(kind, source).await?;
    let first = items(kind, &keys.iter().take(FIRST).cloned().collect::<Vec<_>>()).await;
    let core = app.state::<Core>();
    let db = core.db.lock().unwrap();
    let show = store::make_show(&db, name, kind, Some(source), auto).map_err(|e| e.to_string())?;
    for k in &keys {
        store::mark_seen(&db, show, &k.key).map_err(|e| e.to_string())?;
    }
    // Oldest first, so they read and list in the order they were written.
    for it in first.iter().rev().filter(|it| !it.text.is_empty() || it.link.is_some()) {
        add(app, &db, show, auto, it, source)?;
    }
    drop(db);
    after(app);
    Ok(show)
}

/// What a source holds now, newest first: whole items for a feed, just paths for a folder (read only when wanted).
async fn listing(kind: &str, source: &str) -> Result<Vec<Item>, String> {
    if kind == "feed" {
        return Ok(fetch_feed(source).await?.1);
    }
    let dir = PathBuf::from(source);
    let files = tauri::async_runtime::spawn_blocking(move || folder_files(&dir)).await.map_err(|e| e.to_string())??;
    Ok(files.into_iter().map(|(p, _)| Item { key: p.to_string_lossy().into_owned(), title: file_title(&p), link: None, text: String::new() }).collect())
}

/// Fill in a folder's files with their text; a file that can't be read is left out. Feed items already have theirs.
async fn items(kind: &str, list: &[Item]) -> Vec<Item> {
    if kind == "feed" {
        return list.to_vec();
    }
    let list = list.to_vec();
    tauri::async_runtime::spawn_blocking(move || {
        list.into_iter()
            .filter_map(|it| read::read_file(Path::new(&it.key)).ok().map(|d| Item { title: d.title, text: d.text, ..it }))
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// Every feed and folder show: anything new becomes a read.
pub async fn check(app: &AppHandle) {
    let shows = store::sources(&app.state::<Core>().db.lock().unwrap()).unwrap_or_default();
    for (show, kind, source, auto) in shows {
        let list = match listing(&kind, &source).await {
            Ok(l) => l,
            Err(_) => {
                let why = if kind == "feed" { "Couldn't reach this feed last time." } else { "This folder isn't there any more." };
                let _ = store::set_problem(&app.state::<Core>().db.lock().unwrap(), show, Some(why));
                continue;
            }
        };
        let new: Vec<Item> = {
            let core = app.state::<Core>();
            let db = core.db.lock().unwrap();
            let _ = store::set_problem(&db, show, None);
            list.into_iter().filter(|it| !store::is_seen(&db, show, &it.key).unwrap_or(true)).collect()
        };
        if new.is_empty() {
            continue;
        }
        let ready = items(&kind, &new).await;
        let core = app.state::<Core>();
        let db = core.db.lock().unwrap();
        for it in new.iter() {
            let _ = store::mark_seen(&db, show, &it.key);
        }
        // Oldest first, so they read and list in the order they arrived.
        for it in ready.iter().rev().filter(|it| !it.text.is_empty() || it.link.is_some()) {
            if let Err(e) = add(app, &db, show, auto, it, &source) {
                eprintln!("[sources] {source}: {e}");
            }
        }
    }
    after(app);
}

fn after(app: &AppHandle) {
    read::wake();
    let _ = app.emit("reads", ());
    let _ = app.emit("library", ());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_and_atom() {
        let rss = r#"<?xml version="1.0"?><rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/"><channel><title>A &amp; B</title>
          <item><title>First post</title><link>https://a.b/1</link><guid>g1</guid><description>Short.</description>
            <content:encoded><![CDATA[<h2>Intro</h2><p>Hello <b>there</b>&nbsp;friend.</p><pre>code()</pre><p>Two&rsquo;s here.</p>]]></content:encoded></item>
          <item><title>Second</title><link>https://a.b/2</link><description>Only a summary.</description></item></channel></rss>"#;
        let (t, items) = parse_feed(rss).unwrap();
        assert_eq!(t, "A & B");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].key, "g1");
        assert_eq!(items[0].text, "# Intro\n\nHello there friend.\n\nTwo’s here.");
        assert_eq!((items[1].key.as_str(), items[1].link.as_deref()), ("https://a.b/2", Some("https://a.b/2")));

        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Blog</title><entry><title>E</title><id>urn:1</id>
          <link rel="replies" href="https://x/c"/><link href="https://x/e"/><summary>S</summary><content type="html">&lt;p&gt;Body&lt;/p&gt;</content></entry></feed>"#;
        let (t, items) = parse_feed(atom).unwrap();
        assert_eq!((t.as_str(), items[0].key.as_str(), items[0].link.as_deref(), items[0].text.as_str()), ("Blog", "urn:1", Some("https://x/e"), "Body"));
        assert!(parse_feed("<html></html>").is_err());
    }

    #[test]
    fn entities_decode() {
        assert_eq!(entities("a &amp; b &#8217; &#x2014; &bogus; & c"), "a & b ’ — &bogus; & c");
    }

    #[test]
    fn folders_list_readable_files_newest_first() {
        let dir = std::env::temp_dir().join(format!("zenpod-folder-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let aged = |name: &str, body: &str, secs: u64| {
            std::fs::write(dir.join(name), body).unwrap();
            let t = std::time::SystemTime::now() - std::time::Duration::from_secs(secs);
            std::fs::File::options().write(true).open(dir.join(name)).unwrap().set_modified(t).unwrap();
        };
        aged("old.md", "# Old\n\nText", 600);
        aged("new.txt", "Newer text", 120);
        aged("image.png", "x", 600);
        aged(".hidden.md", "x", 600);
        std::fs::write(dir.join("writing.md"), "half a brie").unwrap(); // just written: left for the next check
        let f = folder_files(&dir).unwrap();
        assert_eq!(f.iter().map(|(p, _)| p.file_name().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>(), ["new.txt", "old.md"]);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(folder_files(&dir).is_err());
    }
}
