use std::sync::LazyLock;

use rss::extension::ExtensionMap;
use serde::Deserialize;

// Some podcast CDNs refuse non-browser clients.
const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";

pub static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(UA)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("http client")
});

pub struct Show {
    pub title: String,
    pub author: Option<String>,
    pub about: Option<String>,
    pub image_url: Option<String>,
}

pub struct Episode {
    pub guid: String,
    pub title: String,
    pub published: Option<i64>,
    pub duration: Option<f64>,
    pub audio_url: String,
    pub image_url: Option<String>,
    pub description: Option<String>,
    pub chapters_url: Option<String>,
    pub transcript_url: Option<String>,
}

pub async fn fetch(url: &str) -> Result<(Show, Vec<Episode>), String> {
    let bytes = HTTP
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("That address could not be reached ({e})."))?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    parse(&bytes)
}

pub fn parse(bytes: &[u8]) -> Result<(Show, Vec<Episode>), String> {
    let ch = rss::Channel::read_from(bytes).map_err(|_| "That address isn't a podcast feed.".to_string())?;
    let it = ch.itunes_ext();
    let show = Show {
        title: ch.title().trim().to_string(),
        author: it.and_then(|i| i.author()).map(str::to_string),
        about: it.and_then(|i| i.summary()).or(Some(ch.description())).map(str::to_string),
        image_url: it.and_then(|i| i.image()).map(str::to_string).or_else(|| ch.image().map(|i| i.url().to_string())),
    };
    let eps = ch
        .items()
        .iter()
        .filter_map(|item| {
            let enc = item.enclosure()?;
            let audio_url = enc.url().to_string();
            let iext = item.itunes_ext();
            Some(Episode {
                guid: item.guid().map(|g| g.value().to_string()).unwrap_or_else(|| audio_url.clone()),
                title: item.title().unwrap_or("Untitled episode").trim().to_string(),
                published: item
                    .pub_date()
                    .and_then(|d| chrono::DateTime::parse_from_rfc2822(d.trim()).ok())
                    .map(|d| d.timestamp()),
                duration: iext.and_then(|i| i.duration()).and_then(parse_duration),
                image_url: iext.and_then(|i| i.image()).map(str::to_string),
                description: item.description().or(item.content()).map(str::to_string),
                chapters_url: podcast_attr(item.extensions(), "chapters", "url"),
                transcript_url: podcast_attr(item.extensions(), "transcript", "url"),
                audio_url,
            })
        })
        .collect();
    Ok((show, eps))
}

// Podcast Namespace tags (<podcast:chapters url=…>), whatever prefix the feed bound it to.
fn podcast_attr(ext: &ExtensionMap, tag: &str, attr: &str) -> Option<String> {
    ext.values()
        .filter_map(|m| m.get(tag))
        .flatten()
        .find_map(|e| e.attrs().get(attr).cloned())
}

fn parse_duration(s: &str) -> Option<f64> {
    s.trim().split(':').try_fold(0.0, |acc, p| p.trim().parse::<f64>().ok().map(|v| acc * 60.0 + v)).filter(|d| *d > 0.0)
}

#[derive(Deserialize)]
struct Search {
    results: Vec<Hit>,
}
#[derive(Deserialize)]
struct Hit {
    #[serde(rename = "collectionName")]
    name: String,
    #[serde(rename = "artistName", default)]
    artist: String,
    #[serde(rename = "feedUrl")]
    feed: Option<String>,
}

fn norm(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_alphanumeric() || *c == ' ').collect()
}

/// Find a show's public feed through Apple's podcast search, by name and (when known) publisher.
/// With a publisher, both must agree; without one, the name alone must.
pub async fn find_feed(name: &str, publisher: &str) -> Option<String> {
    let term: String = name.chars().take(60).collect();
    let res: Search = HTTP
        .get("https://itunes.apple.com/search")
        .query(&[("media", "podcast"), ("entity", "podcast"), ("limit", "10"), ("term", &term)])
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let (n, p) = (norm(name), norm(publisher));
    let prefix = |a: &str, b: &str| {
        let k = a.chars().count().min(b.chars().count()).min(25);
        k > 0 && a.chars().take(k).eq(b.chars().take(k))
    };
    res.results
        .into_iter()
        .filter(|h| h.feed.is_some())
        .find(|h| {
            let (hn, hp) = (norm(&h.name), norm(&h.artist));
            let name_ok = prefix(&n, &hn) || hn.contains(&n);
            let pub_ok = p.is_empty() || hp.contains(&p) || p.contains(&hp) || p.split(' ').any(|w| w.len() > 3 && hp.contains(w));
            name_ok && pub_ok
        })
        .and_then(|h| h.feed)
}

/// The show ID in an Apple Podcasts link, such as podcasts.apple.com/gb/podcast/some-show/id1234567890?i=…
pub fn apple_id(input: &str) -> Option<&str> {
    if !(input.contains("podcasts.apple.com") || input.contains("itunes.apple.com")) {
        return None;
    }
    let rest = &input[input.find("/id")? + 3..];
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    (end > 0).then(|| &rest[..end])
}

/// A show's public feed from its Apple Podcasts ID.
pub async fn lookup_feed(id: &str) -> Option<String> {
    let res: Search = HTTP
        .get("https://itunes.apple.com/lookup")
        .query(&[("id", id), ("entity", "podcast")])
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    res.results.into_iter().find_map(|h| h.feed)
}

/// Feeds the Apple Podcasts app follows on this Mac, read from a copy of its library
/// (the live one is in use, and macOS asks the person before letting us read it).
pub fn apple_library(scratch: &std::path::Path) -> Result<Vec<(String, Option<String>)>, String> {
    let home = std::path::PathBuf::from(std::env::var("HOME").map_err(|e| e.to_string())?);
    let group = home.join("Library/Group Containers/243LU875E5.groups.com.apple.podcasts");
    if !group.exists() {
        return Err("Apple Podcasts has no library on this Mac.".into());
    }
    // macOS keeps Apple's own apps' data behind Full Disk Access and never asks, so open that page for the person.
    let denied = || {
        let _ = std::process::Command::new("open").arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles").status();
        "Apple Podcasts keeps its library behind Full Disk Access. System Settings is open at that page: turn on Zenpod (add it with + if it isn't listed), reopen Zenpod when asked, then choose From Apple Podcasts again. You can turn it off once your shows are in.".to_string()
    };
    std::fs::create_dir_all(scratch).map_err(|e| e.to_string())?;
    let src = group.join("Documents/MTLibrary.sqlite");
    let copy = scratch.join("apple-library.sqlite");
    std::fs::copy(&src, &copy).map_err(|_| denied())?;
    for ext in ["-wal", "-shm"] {
        let (from, to) = (src.with_file_name(format!("MTLibrary.sqlite{ext}")), scratch.join(format!("apple-library.sqlite{ext}")));
        let _ = std::fs::remove_file(&to);
        if from.exists() {
            std::fs::copy(&from, &to).map_err(|_| denied())?;
        }
    }
    let db = rusqlite::Connection::open(&copy).map_err(|e| e.to_string())?;
    let read = |sql: &str| -> rusqlite::Result<Vec<(String, Option<String>)>> {
        let mut q = db.prepare(sql)?;
        let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))?;
        rows.collect()
    };
    // Followed shows only, with their names; older libraries may lack either column.
    let feeds = read("SELECT ZFEEDURL, ZTITLE FROM ZMTPODCAST WHERE ZSUBSCRIBED = 1 AND ZFEEDURL IS NOT NULL")
        .or_else(|_| read("SELECT ZFEEDURL, NULL FROM ZMTPODCAST WHERE ZSUBSCRIBED = 1 AND ZFEEDURL IS NOT NULL"))
        .or_else(|_| read("SELECT ZFEEDURL, NULL FROM ZMTPODCAST WHERE ZFEEDURL IS NOT NULL"))
        .map_err(|_| "Apple Podcasts' library isn't in a form Zenpod understands.".to_string())?;
    drop(db);
    for ext in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(scratch.join(format!("apple-library.sqlite{ext}")));
    }
    Ok(feeds)
}

/// Feed addresses from an OPML file, each with the show's name when the file gives one.
pub fn opml_feeds(text: &str) -> Result<Vec<(String, Option<String>)>, String> {
    let doc = roxmltree::Document::parse(text).map_err(|_| "That file isn't an OPML list.".to_string())?;
    Ok(doc
        .descendants()
        .filter(|n| n.has_tag_name("outline"))
        .filter_map(|n| {
            let url = n.attribute("xmlUrl").or_else(|| n.attribute("xmlurl"))?;
            Some((url.to_string(), n.attribute("text").or_else(|| n.attribute("title")).map(str::to_string)))
        })
        .collect())
}

/// Shows from Spotify's YourLibrary.json (`{"shows": [{name, publisher}]}`) or a bare list of them.
pub fn spotify_shows(text: &str) -> Result<Vec<(String, String)>, String> {
    #[derive(Deserialize)]
    struct S {
        name: String,
        #[serde(default)]
        publisher: String,
    }
    #[derive(Deserialize)]
    struct Lib {
        shows: Vec<S>,
    }
    let v: Vec<S> = serde_json::from_str::<Lib>(text)
        .map(|l| l.shows)
        .or_else(|_| serde_json::from_str::<Vec<S>>(text))
        .map_err(|_| "That file isn't Spotify's YourLibrary.json.".to_string())?;
    Ok(v.into_iter().map(|s| (s.name, s.publisher)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_links() {
        assert_eq!(apple_id("https://podcasts.apple.com/gb/podcast/the-ai-daily-brief/id1680633614?i=1000727"), Some("1680633614"));
        assert_eq!(apple_id("https://podcasts.apple.com/us/podcast/id123"), Some("123"));
        assert_eq!(apple_id("https://itunes.apple.com/podcast/id99"), Some("99"));
        assert_eq!(apple_id("https://example.com/feed/id123"), None);
        assert_eq!(apple_id("https://podcasts.apple.com/us/podcast/no-id"), None);
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("1:02:03"), Some(3723.0));
        assert_eq!(parse_duration("754"), Some(754.0));
        assert_eq!(parse_duration("12:05"), Some(725.0));
        assert_eq!(parse_duration(""), None);
    }

    #[test]
    fn parses_feed_with_namespace_tags() {
        let xml = r#"<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel><title> Show </title><description>d</description><itunes:image href="https://x/i.jpg"/>
          <item><title>Ep</title><guid>g1</guid><pubDate>Tue, 22 Sep 2026 10:00:00 +0000</pubDate>
            <enclosure url="https://x/a.mp3" type="audio/mpeg" length="1"/><itunes:duration>10:00</itunes:duration>
            <podcast:chapters url="https://x/c.json" type="application/json+chapters"/></item>
          <item><title>No audio</title></item></channel></rss>"#;
        let (s, e) = parse(xml.as_bytes()).unwrap();
        assert_eq!(s.title, "Show");
        assert_eq!(s.image_url.as_deref(), Some("https://x/i.jpg"));
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].duration, Some(600.0));
        assert_eq!(e[0].chapters_url.as_deref(), Some("https://x/c.json"));
        assert!(e[0].published.is_some());
    }

    #[test]
    fn reads_imports() {
        assert_eq!(opml_feeds(r#"<opml><body><outline text="a" xmlUrl="https://f"/></body></opml>"#).unwrap(), vec![("https://f".to_string(), Some("a".to_string()))]);
        let lib = spotify_shows(r#"{"shows":[{"name":"Naval","publisher":"Naval"}]}"#).unwrap();
        assert_eq!(lib, vec![("Naval".into(), "Naval".into())]);
        assert_eq!(spotify_shows(r#"[{"name":"X"}]"#).unwrap().len(), 1);
    }
}
