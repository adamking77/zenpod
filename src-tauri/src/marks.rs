//! Notes written out: one Markdown file per episode, in the folder the listener chose. Written whole and swapped in,
//! so anything reading the folder never sees half a file.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::store::{self, EpisodeRow, MarkRow};

/// The notes folder, when one is chosen.
pub fn folder(db: &Connection) -> Option<PathBuf> {
    store::setting(db, "notes_folder").filter(|s| !s.is_empty()).map(PathBuf::from)
}

/// Lowercase ASCII words joined by hyphens.
fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-');
    out.chars().take(90).collect::<String>().trim_end_matches('-').to_string()
}

fn clock(t: f64) -> String {
    let v = t.max(0.0) as u64;
    let (h, m, s) = (v / 3600, (v % 3600) / 60, v % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// `<show>-<episode>.md`; an episode whose name another episode's file already has gets its id added.
pub fn file_for(dir: &Path, e: &EpisodeRow) -> PathBuf {
    let base = format!("{}-{}", slug(&e.show_title), slug(&e.title));
    let plain = dir.join(format!("{base}.md"));
    let ours = |p: &Path| std::fs::read_to_string(p).map(|t| t.contains(&format!("\nepisode_id: {}\n", e.id))).unwrap_or(true);
    if !plain.exists() || ours(&plain) { plain } else { dir.join(format!("{base}-{}.md", e.id)) }
}

/// Where a note's time in the Markdown points: Zenpod, at that episode and second.
pub fn link(episode: i64, start: f64) -> String {
    format!("zenpod://episode/{episode}?t={}", start.max(0.0).floor() as u64)
}

/// A `zenpod://episode/<id>?t=<seconds>` link, read back: the episode and where to start.
pub fn parse_link(url: &str) -> Option<(i64, f64)> {
    let rest = url.strip_prefix("zenpod://episode/")?;
    let (id, query) = rest.split_once('?').unwrap_or((rest, ""));
    let id = id.trim_end_matches('/').parse().ok()?;
    let t = query.split('&').find_map(|kv| kv.strip_prefix("t=")).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    Some((id, t.max(0.0)))
}

pub fn markdown(e: &EpisodeRow, marks: &[MarkRow]) -> String {
    let when = e.published.and_then(|t| chrono::DateTime::from_timestamp(t, 0));
    let mut out = format!("---\ntitle: \"{}\"\nshow: \"{}\"\n", e.title.replace('"', "\\\""), e.show_title.replace('"', "\\\""));
    if let Some(d) = when {
        out += &format!("published: {}\n", d.format("%Y-%m-%d"));
    }
    out += &format!("episode_id: {}\n---\n\n# {}\n\n{}", e.id, e.title, e.show_title);
    if let Some(d) = when {
        out += &format!(" · {}", d.format("%-d %b"));
    }
    out += "\n\n";
    for m in marks {
        // The times open Zenpod at the note: zenpod://episode/<id>?t=<seconds>.
        out += &format!("## [{}–{}]({})", clock(m.start), clock(m.end), link(e.id, m.start));
        if let Some(c) = m.chapter.as_deref().filter(|c| !c.is_empty()) {
            out += &format!(" · {c}");
        }
        out += "\n\n";
        if let Some(q) = m.quote.as_deref().filter(|q| !q.is_empty()) {
            out += &format!("> {q}\n\n");
        }
        if !m.note.is_empty() {
            out += &format!("{}\n\n", m.note);
        }
    }
    if marks.is_empty() {
        out += "No notes yet.\n";
    }
    out
}

/// Rewrite one episode's file.
pub fn write(db: &Connection, dir: &Path, episode: i64) -> Result<PathBuf, String> {
    let e = store::episode(db, episode).map_err(|e| e.to_string())?.ok_or("That episode is gone.")?;
    let marks = store::marks(db, episode).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = file_for(dir, &e);
    let tmp = path.with_extension("md.part");
    std::fs::write(&tmp, markdown(&e, &marks)).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Write every episode that has notes; returns how many files were written.
pub fn write_all(db: &Connection, dir: &Path) -> usize {
    let ids = store::marked_episodes(db).unwrap_or_default();
    ids.into_iter().filter(|&id| write(db, dir, id).map_err(|e| eprintln!("[notes] {e}")).is_ok()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep() -> EpisodeRow {
        EpisodeRow {
            id: 7, show_id: 1, show_title: "What Works".into(), title: "We're Not Late (Or, Rethinking the Long-Term)".into(),
            published: Some(1_717_459_200), duration: Some(1233.0), position: 0.0, played: false, kept: false, offline: false, image_url: None,
        }
    }
    fn mark(start: f64, note: &str, quote: Option<&str>) -> MarkRow {
        MarkRow {
            id: 1, episode_id: 7, start, end: start + 30.0, note: note.into(), quote: quote.map(Into::into), chapter: Some("Time Flies".into()),
            created: 0, show_id: 1, show_title: "What Works".into(), episode_title: "x".into(), image_url: None,
        }
    }

    #[test]
    fn links_go_both_ways() {
        assert_eq!(link(7, 282.9), "zenpod://episode/7?t=282");
        assert_eq!(parse_link("zenpod://episode/7?t=282"), Some((7, 282.0)));
        assert_eq!(parse_link("zenpod://episode/7"), Some((7, 0.0)));
        assert_eq!(parse_link("zenpod://episode/x?t=1"), None);
        assert_eq!(parse_link("https://example.com"), None);
    }

    #[test]
    fn names_and_times() {
        assert_eq!(slug("We're Not Late (Or, Rethinking the Long-Term)"), "we-re-not-late-or-rethinking-the-long-term");
        assert_eq!(clock(282.4), "4:42");
        assert_eq!(clock(3725.0), "1:02:05");
    }

    #[test]
    fn markdown_shape() {
        let md = markdown(&ep(), &[mark(282.0, "Temporal bandwidth, for the workshop", Some("To be on time is to be late."))]);
        assert!(md.starts_with("---\ntitle: \"We're Not Late (Or, Rethinking the Long-Term)\"\nshow: \"What Works\"\npublished: 2024-06-04\nepisode_id: 7\n---\n"));
        assert!(md.contains("\n# We're Not Late (Or, Rethinking the Long-Term)\n\nWhat Works · 4 Jun\n\n## [4:42–5:12](zenpod://episode/7?t=282) · Time Flies\n\n> To be on time is to be late.\n\nTemporal bandwidth, for the workshop\n"));
        assert!(markdown(&ep(), &[]).ends_with("No notes yet.\n"));
    }

    #[test]
    fn writes_whole_files_and_keeps_names_apart() {
        let dir = std::env::temp_dir().join(format!("zenpod-notes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = file_for(&dir, &ep());
        assert_eq!(a.file_name().unwrap(), "what-works-we-re-not-late-or-rethinking-the-long-term.md");
        std::fs::write(&a, markdown(&ep(), &[])).unwrap();
        assert_eq!(file_for(&dir, &ep()), a, "an episode keeps its own file");
        let mut other = ep();
        other.id = 9;
        assert!(file_for(&dir, &other).ends_with("what-works-we-re-not-late-or-rethinking-the-long-term-9.md"), "a second episode with the same name gets its id");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn notes_live_and_die_with_their_episode() {
        let db = store::open(Path::new(":memory:")).unwrap();
        db.execute_batch("insert into shows(id, feed_url, title) values(1, 'f', 'What Works');
          insert into episodes(id, show_id, guid, title, audio_url) values(7, 1, 'g', 'Ep', 'a');").unwrap();
        let m = store::add_mark(&db, 7, 282.0, 312.0, Some("Said."), Some("Time Flies")).unwrap();
        assert_eq!((m.start, m.end, m.note.as_str(), m.show_title.as_str()), (282.0, 312.0, "", "What Works"));
        store::set_mark_note(&db, m.id, "Mine").unwrap();
        assert_eq!(store::marks(&db, 7).unwrap()[0].note, "Mine");
        assert_eq!(store::mark_episode(&db, m.id).unwrap(), Some(7));
        assert_eq!(store::marked_episodes(&db).unwrap(), vec![7]);
        db.execute("delete from episodes where id = 7", []).unwrap();
        assert!(store::all_marks(&db).unwrap().is_empty(), "an episode's notes go with it");
    }
}
