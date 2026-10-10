use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::feed::{Episode, Show};

pub fn open(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.execute_batch(
        "pragma journal_mode = wal;
         pragma foreign_keys = on;
         create table if not exists shows(
           id integer primary key, feed_url text unique not null, title text not null,
           author text, about text, image_url text, spotify_only integer not null default 0,
           refreshed_at integer);
         create table if not exists episodes(
           id integer primary key, show_id integer not null references shows(id) on delete cascade,
           guid text not null, title text not null, published integer, duration real,
           audio_url text not null, image_url text, description text,
           chapters_url text, transcript_url text,
           position real not null default 0, played integer not null default 0,
           peaks blob, local_path text, kept integer not null default 0,
           unique(show_id, guid));
         create index if not exists episodes_published on episodes(published desc);
         create table if not exists settings(key text primary key, value text not null);
         create table if not exists marks(
           id integer primary key, episode_id integer not null references episodes(id) on delete cascade,
           start_at real not null, end_at real not null, note text not null default '',
           quote text, chapter text, created integer not null default (unixepoch()));
         create index if not exists marks_episode on marks(episode_id, start_at);
         create table if not exists reads(
           id integer primary key, show_id integer not null references shows(id) on delete cascade,
           episode_id integer references episodes(id) on delete set null,
           title text not null, source text not null, source_key text,
           chars integer not null, state text not null,
           pieces integer not null default 0, done integer not null default 0,
           error text, created integer not null default (unixepoch()));
         create index if not exists reads_show on reads(show_id, created);
         create unique index if not exists reads_source on reads(show_id, source_key);",
    )?;
    // Shows you make: what kind, where new episodes come from, whether they're read automatically, and their own voice.
    let have: Vec<String> = db.prepare("select name from pragma_table_info('shows')")?.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    for (col, ty) in [("kind", "text"), ("source", "text"), ("auto", "integer not null default 0"), ("voice", "text")] {
        if !have.iter().any(|h| h == col) {
            db.execute_batch(&format!("alter table shows add column {col} {ty}"))?;
        }
    }
    // A reading cut off by quitting carries on from its last finished piece.
    db.execute("update reads set state = 'queued' where state = 'reading'", [])?;
    Ok(db)
}

pub fn upsert_show(db: &Connection, feed_url: &str, s: &Show) -> rusqlite::Result<i64> {
    db.query_row(
        "insert into shows(feed_url, title, author, about, image_url, refreshed_at)
         values(?1, ?2, ?3, ?4, ?5, unixepoch())
         on conflict(feed_url) do update set title = ?2, author = ?3, about = ?4,
           image_url = ?5, spotify_only = 0, refreshed_at = unixepoch()
         returning id",
        params![feed_url, s.title, s.author, s.about, s.image_url],
        |r| r.get(0),
    )
}

pub fn upsert_episodes(db: &mut Connection, show_id: i64, eps: &[Episode]) -> rusqlite::Result<()> {
    let tx = db.transaction()?;
    {
        let mut st = tx.prepare(
            "insert into episodes(show_id, guid, title, published, duration, audio_url, image_url,
               description, chapters_url, transcript_url)
             values(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             on conflict(show_id, guid) do update set title = ?3, published = ?4,
               duration = coalesce(?5, duration), audio_url = ?6, image_url = ?7, description = ?8,
               chapters_url = ?9, transcript_url = ?10",
        )?;
        for e in eps {
            st.execute(params![
                show_id, e.guid, e.title, e.published, e.duration, e.audio_url, e.image_url,
                e.description, e.chapters_url, e.transcript_url
            ])?;
        }
    }
    tx.commit()
}

pub fn add_spotify_only(db: &Connection, name: &str, publisher: &str) -> rusqlite::Result<()> {
    db.execute(
        "insert into shows(feed_url, title, author, spotify_only) values(?1, ?2, ?3, 1)
         on conflict(feed_url) do nothing",
        params![format!("spotify:{}", name.to_lowercase()), name, publisher],
    )?;
    Ok(())
}

pub fn feeds(db: &Connection) -> rusqlite::Result<Vec<(i64, String)>> {
    let mut st = db.prepare("select id, feed_url from shows where spotify_only = 0 and kind is null")?;
    let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    rows.collect()
}

pub fn has_title(db: &Connection, title: &str) -> rusqlite::Result<bool> {
    db.query_row("select exists(select 1 from shows where lower(title) = lower(?1))", [title], |r| r.get(0))
}

#[derive(Serialize)]
pub struct ShowRow {
    pub id: i64,
    pub title: String,
    pub author: Option<String>,
    pub about: Option<String>,
    pub image_url: Option<String>,
    pub spotify_only: bool,
    pub fresh: i64,
    pub latest: Option<i64>,
    /// `read-to-me`, `items`, `feed` or `folder` for a show you made; none for a podcast.
    pub kind: Option<String>,
    pub source: Option<String>,
    pub auto: bool,
    pub voice: Option<String>,
}

pub fn shows(db: &Connection) -> rusqlite::Result<Vec<ShowRow>> {
    let mut st = db.prepare(
        "select s.id, s.title, s.author, s.about, s.image_url, s.spotify_only,
           (select count(*) from episodes e where e.show_id = s.id and e.played = 0 and e.position = 0
              and e.published > unixepoch() - 7 * 86400),
           (select max(published) from episodes e where e.show_id = s.id),
           s.kind, s.source, s.auto, s.voice
         from shows s order by s.spotify_only, lower(s.title)",
    )?;
    let rows = st.query_map([], |r| {
        Ok(ShowRow {
            id: r.get(0)?,
            title: r.get(1)?,
            author: r.get(2)?,
            about: r.get(3)?,
            image_url: r.get(4)?,
            spotify_only: r.get(5)?,
            fresh: r.get(6)?,
            latest: r.get(7)?,
            kind: r.get(8)?,
            source: r.get(9)?,
            auto: r.get(10)?,
            voice: r.get(11)?,
        })
    })?;
    rows.collect()
}

#[derive(Serialize, Clone)]
pub struct EpisodeRow {
    pub id: i64,
    pub show_id: i64,
    pub show_title: String,
    pub title: String,
    pub published: Option<i64>,
    pub duration: Option<f64>,
    pub position: f64,
    pub played: bool,
    pub kept: bool,
    pub offline: bool,
    pub image_url: Option<String>,
    /// Set when the episode belongs to a show you made.
    pub show_kind: Option<String>,
}

const EPISODE_COLS: &str = "e.id, e.show_id, s.title, e.title, e.published, e.duration, e.position,
  e.played, e.kept, e.local_path is not null, coalesce(s.image_url, e.image_url), s.kind";

fn episode_row(r: &rusqlite::Row) -> rusqlite::Result<EpisodeRow> {
    Ok(EpisodeRow {
        id: r.get(0)?,
        show_id: r.get(1)?,
        show_title: r.get(2)?,
        title: r.get(3)?,
        published: r.get(4)?,
        duration: r.get(5)?,
        position: r.get(6)?,
        played: r.get(7)?,
        kept: r.get(8)?,
        offline: r.get(9)?,
        image_url: r.get(10)?,
        show_kind: r.get(11)?,
    })
}

pub fn newest(db: &Connection, limit: i64) -> rusqlite::Result<Vec<EpisodeRow>> {
    let mut st = db.prepare(&format!(
        "select {EPISODE_COLS} from episodes e join shows s on s.id = e.show_id
         order by e.published desc limit ?1"
    ))?;
    let rows = st.query_map([limit], episode_row)?;
    rows.collect()
}

pub fn show_episodes(db: &Connection, show_id: i64) -> rusqlite::Result<Vec<EpisodeRow>> {
    let mut st = db.prepare(&format!(
        "select {EPISODE_COLS} from episodes e join shows s on s.id = e.show_id
         where e.show_id = ?1 order by e.published desc"
    ))?;
    let rows = st.query_map([show_id], episode_row)?;
    rows.collect()
}

pub fn episode(db: &Connection, id: i64) -> rusqlite::Result<Option<EpisodeRow>> {
    db.query_row(
        &format!("select {EPISODE_COLS} from episodes e join shows s on s.id = e.show_id where e.id = ?1"),
        [id],
        episode_row,
    )
    .optional()
}

pub fn setting(db: &Connection, key: &str) -> Option<String> {
    db.query_row("select value from settings where key = ?1", [key], |r| r.get(0))
        .optional()
        .ok()
        .flatten()
}

pub fn set_setting(db: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    db.execute(
        "insert into settings(key, value) values(?1, ?2) on conflict(key) do update set value = ?2",
        params![key, value],
    )?;
    Ok(())
}

/// (downloaded file, remote address) for an episode.
pub fn audio_source(db: &Connection, id: i64) -> rusqlite::Result<(Option<String>, String)> {
    db.query_row("select local_path, audio_url from episodes where id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?)))
}

pub fn show_image(db: &Connection, show: i64) -> rusqlite::Result<Option<String>> {
    db.query_row("select image_url from shows where id = ?1", [show], |r| r.get(0))
}

pub fn save_position(db: &Connection, id: i64, position: f64) -> rusqlite::Result<()> {
    db.execute("update episodes set position = ?2 where id = ?1", params![id, position])?;
    Ok(())
}

pub fn mark_played(db: &Connection, id: i64) -> rusqlite::Result<()> {
    db.execute("update episodes set played = 1, position = 0 where id = ?1", [id])?;
    Ok(())
}

pub fn set_duration(db: &Connection, id: i64, duration: f64) -> rusqlite::Result<()> {
    db.execute("update episodes set duration = ?2 where id = ?1", params![id, duration])?;
    Ok(())
}

pub fn set_local(db: &Connection, id: i64, path: Option<&str>) -> rusqlite::Result<()> {
    db.execute("update episodes set local_path = ?2 where id = ?1", params![id, path])?;
    Ok(())
}

pub fn peaks(db: &Connection, id: i64) -> rusqlite::Result<Option<Vec<u8>>> {
    db.query_row("select peaks from episodes where id = ?1", [id], |r| r.get(0))
}

pub fn set_peaks(db: &Connection, id: i64, blob: &[u8]) -> rusqlite::Result<()> {
    db.execute("update episodes set peaks = ?2 where id = ?1", params![id, blob])?;
    Ok(())
}

pub fn description(db: &Connection, id: i64) -> rusqlite::Result<Option<String>> {
    db.query_row("select description from episodes where id = ?1", [id], |r| r.get(0))
}

/// Remove a show and its episodes; returns the files it leaves behind.
pub fn unfollow(db: &Connection, show: i64) -> rusqlite::Result<Vec<String>> {
    let files: Vec<String> = db
        .prepare("select local_path from episodes where show_id = ?1 and local_path is not null")?
        .query_map([show], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    db.execute("delete from shows where id = ?1", [show])?;
    Ok(files)
}

pub fn chapters_url(db: &Connection, id: i64) -> rusqlite::Result<Option<String>> {
    db.query_row("select chapters_url from episodes where id = ?1", [id], |r| r.get(0))
}

pub fn set_kept(db: &Connection, id: i64, kept: bool) -> rusqlite::Result<()> {
    db.execute("update episodes set kept = ?2 where id = ?1", params![id, kept])?;
    Ok(())
}

/// Saved copies nobody asked to keep, of episodes already heard.
pub fn spent_copies(db: &Connection) -> rusqlite::Result<Vec<(i64, String)>> {
    db.prepare("select id, local_path from episodes where local_path is not null and kept = 0 and played = 1")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect()
}

pub fn transcript_url(db: &Connection, id: i64) -> rusqlite::Result<Option<String>> {
    db.query_row("select transcript_url from episodes where id = ?1", [id], |r| r.get(0))
}

/// A note: the stretch of an episode it keeps, the listener's words, and what was said there. Called a mark in
/// code because the show's own notes are already `description` / `episode_notes`.
#[derive(Serialize, Clone)]
pub struct MarkRow {
    pub id: i64,
    pub episode_id: i64,
    pub start: f64,
    pub end: f64,
    pub note: String,
    pub quote: Option<String>,
    pub chapter: Option<String>,
    pub created: i64,
    pub show_id: i64,
    pub show_title: String,
    pub episode_title: String,
    pub image_url: Option<String>,
}

const MARK_FROM: &str = "select m.id, m.episode_id, m.start_at, m.end_at, m.note, m.quote, m.chapter, m.created,
  e.show_id, s.title, e.title, coalesce(s.image_url, e.image_url)
  from marks m join episodes e on e.id = m.episode_id join shows s on s.id = e.show_id";

fn mark_row(r: &rusqlite::Row) -> rusqlite::Result<MarkRow> {
    Ok(MarkRow {
        id: r.get(0)?,
        episode_id: r.get(1)?,
        start: r.get(2)?,
        end: r.get(3)?,
        note: r.get(4)?,
        quote: r.get(5)?,
        chapter: r.get(6)?,
        created: r.get(7)?,
        show_id: r.get(8)?,
        show_title: r.get(9)?,
        episode_title: r.get(10)?,
        image_url: r.get(11)?,
    })
}

pub fn add_mark(db: &Connection, episode: i64, start: f64, end: f64, quote: Option<&str>, chapter: Option<&str>) -> rusqlite::Result<MarkRow> {
    let id: i64 = db.query_row(
        "insert into marks(episode_id, start_at, end_at, quote, chapter) values(?1, ?2, ?3, ?4, ?5) returning id",
        params![episode, start, end, quote, chapter],
        |r| r.get(0),
    )?;
    db.query_row(&format!("{MARK_FROM} where m.id = ?1"), [id], mark_row)
}

pub fn marks(db: &Connection, episode: i64) -> rusqlite::Result<Vec<MarkRow>> {
    let mut st = db.prepare(&format!("{MARK_FROM} where m.episode_id = ?1 order by m.start_at"))?;
    let rows = st.query_map([episode], mark_row)?;
    rows.collect()
}

pub fn all_marks(db: &Connection) -> rusqlite::Result<Vec<MarkRow>> {
    let mut st = db.prepare(&format!("{MARK_FROM} order by m.created desc, m.id desc"))?;
    let rows = st.query_map([], mark_row)?;
    rows.collect()
}

/// The episode a note belongs to, if the note exists.
pub fn mark_episode(db: &Connection, id: i64) -> rusqlite::Result<Option<i64>> {
    db.query_row("select episode_id from marks where id = ?1", [id], |r| r.get(0)).optional()
}

pub fn set_mark_note(db: &Connection, id: i64, note: &str) -> rusqlite::Result<()> {
    db.execute("update marks set note = ?2 where id = ?1", params![id, note])?;
    Ok(())
}

pub fn remove_mark(db: &Connection, id: i64) -> rusqlite::Result<()> {
    db.execute("delete from marks where id = ?1", [id])?;
    Ok(())
}

pub fn marked_episodes(db: &Connection) -> rusqlite::Result<Vec<i64>> {
    db.prepare("select distinct episode_id from marks")?.query_map([], |r| r.get(0))?.collect()
}

// ---------- Read to me: your shows and what's on its way to becoming an episode ----------

pub const READ_TO_ME: &str = "zenpod:show:read-to-me";

/// The Read to me show, made the first time it's needed.
pub fn read_to_me(db: &Connection) -> rusqlite::Result<i64> {
    db.query_row(
        "insert into shows(feed_url, title, kind) values(?1, 'Read to me', 'read-to-me')
         on conflict(feed_url) do update set kind = 'read-to-me' returning id",
        [READ_TO_ME],
        |r| r.get(0),
    )
}

/// A show of yours. `source` is a feed address or folder path, or none for things you add one at a time.
pub fn make_show(db: &Connection, title: &str, kind: &str, source: Option<&str>, auto: bool) -> rusqlite::Result<i64> {
    let uuid = format!("{:x}{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0), std::process::id());
    db.query_row(
        "insert into shows(feed_url, title, kind, source, auto, refreshed_at) values(?1, ?2, ?3, ?4, ?5, unixepoch()) returning id",
        params![format!("zenpod:show:{uuid}"), title, kind, source, auto],
        |r| r.get(0),
    )
}

pub fn set_show(db: &Connection, id: i64, title: Option<&str>, auto: Option<bool>, voice: Option<&str>) -> rusqlite::Result<()> {
    if let Some(t) = title {
        db.execute("update shows set title = ?2 where id = ?1 and kind is not null", params![id, t])?;
    }
    if let Some(a) = auto {
        db.execute("update shows set auto = ?2 where id = ?1 and kind is not null", params![id, a])?;
    }
    if let Some(v) = voice {
        db.execute("update shows set voice = nullif(?2, '') where id = ?1 and kind is not null", params![id, v])?;
    }
    Ok(())
}

pub fn show_voice(db: &Connection, id: i64) -> Option<String> {
    db.query_row("select voice from shows where id = ?1", [id], |r| r.get(0)).optional().ok().flatten().flatten()
}

#[derive(Serialize, Clone)]
pub struct ReadRow {
    pub id: i64,
    pub show_id: i64,
    pub show_title: String,
    pub episode_id: Option<i64>,
    pub title: String,
    pub source: String,
    pub chars: i64,
    /// waiting, queued, reading, failed or done
    pub state: String,
    pub pieces: i64,
    pub done: i64,
    pub error: Option<String>,
    pub created: i64,
}

const READ_FROM: &str = "select r.id, r.show_id, s.title, r.episode_id, r.title, r.source, r.chars, r.state, r.pieces, r.done, r.error, r.created
  from reads r join shows s on s.id = r.show_id";

fn read_row(r: &rusqlite::Row) -> rusqlite::Result<ReadRow> {
    Ok(ReadRow {
        id: r.get(0)?,
        show_id: r.get(1)?,
        show_title: r.get(2)?,
        episode_id: r.get(3)?,
        title: r.get(4)?,
        source: r.get(5)?,
        chars: r.get(6)?,
        state: r.get(7)?,
        pieces: r.get(8)?,
        done: r.get(9)?,
        error: r.get(10)?,
        created: r.get(11)?,
    })
}

pub fn add_read(db: &Connection, show: i64, title: &str, source: &str, source_key: Option<&str>, chars: i64, state: &str) -> rusqlite::Result<Option<i64>> {
    db.query_row(
        "insert into reads(show_id, title, source, source_key, chars, state) values(?1, ?2, ?3, ?4, ?5, ?6)
         on conflict(show_id, source_key) do nothing returning id",
        params![show, title, source, source_key, chars, state],
        |r| r.get(0),
    )
    .optional()
}

/// Everything not yet an episode, oldest first: what's being read, waiting its turn, waiting for you, or stopped.
pub fn pending_reads(db: &Connection) -> rusqlite::Result<Vec<ReadRow>> {
    db.prepare(&format!("{READ_FROM} where r.state != 'done' order by r.id"))?.query_map([], read_row)?.collect()
}

pub fn read(db: &Connection, id: i64) -> rusqlite::Result<Option<ReadRow>> {
    db.query_row(&format!("{READ_FROM} where r.id = ?1"), [id], read_row).optional()
}

pub fn next_queued(db: &Connection) -> rusqlite::Result<Option<ReadRow>> {
    db.query_row(&format!("{READ_FROM} where r.state = 'queued' order by r.id limit 1"), [], read_row).optional()
}

pub fn set_read_state(db: &Connection, id: i64, state: &str, error: Option<&str>) -> rusqlite::Result<()> {
    db.execute("update reads set state = ?2, error = ?3 where id = ?1", params![id, state, error])?;
    Ok(())
}

pub fn set_read_progress(db: &Connection, id: i64, done: i64, pieces: i64) -> rusqlite::Result<()> {
    db.execute("update reads set done = ?2, pieces = ?3 where id = ?1", params![id, done, pieces])?;
    Ok(())
}

pub fn remove_read(db: &Connection, id: i64) -> rusqlite::Result<()> {
    db.execute("delete from reads where id = ?1", [id])?;
    Ok(())
}

/// The finished episode of a read, already downloaded and kept: nothing streams, nothing is tidied away.
pub struct Made<'a> {
    pub show: i64,
    pub read: i64,
    pub title: &'a str,
    pub duration: f64,
    pub path: &'a str,
    pub description: &'a str,
    pub transcript: &'a str,
    pub chapters: Option<&'a str>,
}

pub fn finish_read(db: &Connection, m: &Made) -> rusqlite::Result<i64> {
    let id: i64 = db.query_row(
        "insert into episodes(show_id, guid, title, published, duration, audio_url, description, chapters_url, transcript_url, local_path, kept)
         values(?1, ?2, ?3, unixepoch(), ?4, ?5, ?6, ?7, ?8, ?9, 1)
         on conflict(show_id, guid) do update set title = ?3, duration = ?4, audio_url = ?5, description = ?6,
           chapters_url = ?7, transcript_url = ?8, local_path = ?9, kept = 1, peaks = null
         returning id",
        params![m.show, format!("read:{}", m.read), m.title, m.duration, format!("file://{}", m.path), m.description, m.chapters, m.transcript, m.path],
        |r| r.get(0),
    )?;
    db.execute("update reads set state = 'done', episode_id = ?2, error = null where id = ?1", params![m.read, id])?;
    Ok(id)
}

/// Remove one episode of yours; returns its audio file.
pub fn delete_episode(db: &Connection, id: i64) -> rusqlite::Result<Option<String>> {
    let path: Option<String> = db.query_row("select local_path from episodes where id = ?1", [id], |r| r.get(0)).optional()?.flatten();
    db.execute("delete from reads where episode_id = ?1", [id])?;
    db.execute("delete from episodes where id = ?1 and show_id in (select id from shows where kind is not null)", [id])?;
    Ok(path)
}

/// The reads of a show, for tidying their files when it goes.
pub fn show_read_ids(db: &Connection, show: i64) -> rusqlite::Result<Vec<i64>> {
    db.prepare("select id from reads where show_id = ?1")?.query_map([show], |r| r.get(0))?.collect()
}

pub fn show_episode_ids(db: &Connection, show: i64) -> rusqlite::Result<Vec<i64>> {
    db.prepare("select id from episodes where show_id = ?1")?.query_map([show], |r| r.get(0))?.collect()
}

#[cfg(test)]
mod read_tests {
    use super::*;

    fn mem() -> Connection {
        let p = std::env::temp_dir().join(format!("zenpod-store-{}-{:?}.db", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_file(&p);
        open(&p).unwrap()
    }

    #[test]
    fn your_shows_stay_out_of_refresh_and_reads_become_episodes() {
        let db = mem();
        let home = read_to_me(&db).unwrap();
        assert_eq!(read_to_me(&db).unwrap(), home, "made once");
        let mine = make_show(&db, "Research", "items", None, false).unwrap();
        assert!(feeds(&db).unwrap().is_empty(), "refresh never sees your shows");

        let r = add_read(&db, mine, "An essay", "example.com", Some("k1"), 9000, "queued").unwrap().unwrap();
        assert!(add_read(&db, mine, "An essay", "example.com", Some("k1"), 9000, "queued").unwrap().is_none(), "a source is read once per show");
        assert_eq!(next_queued(&db).unwrap().unwrap().id, r);

        let e = finish_read(&db, &Made { show: mine, read: r, title: "An essay", duration: 61.5, path: "/tmp/x.m4a", description: "", transcript: "file:///t", chapters: Some("file:///c") }).unwrap();
        let ep = episode(&db, e).unwrap().unwrap();
        assert_eq!((ep.show_kind.as_deref(), ep.offline, ep.kept), (Some("items"), true, true));
        assert!(pending_reads(&db).unwrap().is_empty());
        assert!(spent_copies(&db).unwrap().is_empty(), "a made episode is never tidied away");

        assert_eq!(delete_episode(&db, e).unwrap().as_deref(), Some("/tmp/x.m4a"));
        assert!(episode(&db, e).unwrap().is_none());
        assert!(read(&db, r).unwrap().is_none());
    }

    #[test]
    fn a_podcast_episode_cant_be_deleted_as_yours() {
        let db = mem();
        let show = upsert_show(&db, "https://feed", &Show { title: "Pod".into(), author: None, about: None, image_url: None }).unwrap();
        db.execute("insert into episodes(show_id, guid, title, audio_url) values(?1, 'g', 'Ep', 'https://a')", [show]).unwrap();
        let id: i64 = db.query_row("select id from episodes", [], |r| r.get(0)).unwrap();
        delete_episode(&db, id).unwrap();
        assert!(episode(&db, id).unwrap().is_some());
    }
}
