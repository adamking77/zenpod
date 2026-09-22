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
         create table if not exists settings(key text primary key, value text not null);",
    )?;
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
    let mut st = db.prepare("select id, feed_url from shows where spotify_only = 0")?;
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
}

pub fn shows(db: &Connection) -> rusqlite::Result<Vec<ShowRow>> {
    let mut st = db.prepare(
        "select s.id, s.title, s.author, s.about, s.image_url, s.spotify_only,
           (select count(*) from episodes e where e.show_id = s.id and e.played = 0 and e.position = 0
              and e.published > unixepoch() - 7 * 86400),
           (select max(published) from episodes e where e.show_id = s.id)
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
    pub image_url: Option<String>,
}

const EPISODE_COLS: &str = "e.id, e.show_id, s.title, e.title, e.published, e.duration, e.position,
  e.played, e.kept, coalesce(s.image_url, e.image_url)";

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
        image_url: r.get(9)?,
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
