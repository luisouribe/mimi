//! Local dictation history backed by SQLite, with accent-insensitive full-text search.
//!
//! Everything lives in one small database file. The `dictations` table is the
//! source of truth and an FTS5 external-content index mirrors its text, kept in
//! sync by triggers so callers never touch the index directly.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::snippets::{self, Snippet};

/// Bump together with a new step in `migrate` when the schema changes.
const SCHEMA_VERSION: i64 = 2;

const COLUMNS: &str = "id, text, app_name, created_at_ms, word_count, speech_seconds, pasted";

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct HistoryEntry {
    pub id: i64,
    pub text: String,
    pub app_name: String,
    pub created_at_ms: i64,
    pub word_count: i64,
    pub speech_seconds: f64,
    pub pasted: bool,
}

pub struct NewEntry<'a> {
    pub text: &'a str,
    pub app_name: &'a str,
    pub word_count: i64,
    pub speech_seconds: f64,
    pub pasted: bool,
}

pub struct History {
    conn: Mutex<Connection>,
}

// The app shares one History across threads; fail the build if that ever stops holding.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<History>();
};

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn row_to_entry(row: &Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        text: row.get(1)?,
        app_name: row.get(2)?,
        created_at_ms: row.get(3)?,
        word_count: row.get(4)?,
        speech_seconds: row.get(5)?,
        pasted: row.get::<_, i64>(6)? != 0,
    })
}

/// SQLite takes i64 limits; a huge usize simply means "no practical limit".
fn sql_limit(limit: usize) -> i64 {
    i64::try_from(limit).unwrap_or(i64::MAX)
}

/// Turns raw user input into a safe FTS5 expression: every word is a quoted
/// prefix term and all must match. Returns `None` when nothing searchable remains.
fn build_fts_query(query: &str) -> Option<String> {
    // FTS5 reads the expression as a C string, so an embedded NUL would cut the
    // quoting short and cause a syntax error; treat it as a word separator.
    let cleaned = query.replace('\0', " ");
    let terms: Vec<String> = cleaned
        .split_whitespace()
        // A word with no letters or digits has no tokens, and an empty FTS5
        // phrase is not a reliable match expression, so drop it.
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .map(|w| format!("\"{}\"*", w.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        None
    } else {
        // Implicit AND between space-separated terms.
        Some(terms.join(" "))
    }
}

/// Japanese and Chinese have no spaces, so the word index only sees whole runs of text.
fn has_unspaced_language(query: &str) -> bool {
    query.chars().any(|c| c >= '\u{3000}')
}

/// A search pattern that finds this text anywhere, with its wildcard characters made literal.
fn anywhere_pattern(term: &str) -> String {
    let escaped = term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

fn migrate(conn: &Connection) -> Result<(), String> {
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(err)?;
    if version > SCHEMA_VERSION {
        return Err(format!(
            "history database is version {version}, this app only understands up to {SCHEMA_VERSION}"
        ));
    }
    if version < 1 {
        // One transaction so a crash cannot leave a half-built schema.
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE IF NOT EXISTS dictations (
                 id             INTEGER PRIMARY KEY AUTOINCREMENT,
                 text           TEXT    NOT NULL,
                 app_name       TEXT    NOT NULL DEFAULT '',
                 created_at_ms  INTEGER NOT NULL,
                 word_count     INTEGER NOT NULL DEFAULT 0,
                 speech_seconds REAL    NOT NULL DEFAULT 0,
                 pasted         INTEGER NOT NULL DEFAULT 0
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS dictations_fts USING fts5(
                 text,
                 content = 'dictations',
                 content_rowid = 'id',
                 tokenize = 'unicode61 remove_diacritics 2'
             );
             CREATE TRIGGER IF NOT EXISTS dictations_ai AFTER INSERT ON dictations BEGIN
                 INSERT INTO dictations_fts(rowid, text) VALUES (new.id, new.text);
             END;
             CREATE TRIGGER IF NOT EXISTS dictations_ad AFTER DELETE ON dictations BEGIN
                 INSERT INTO dictations_fts(dictations_fts, rowid, text)
                 VALUES ('delete', old.id, old.text);
             END;
             PRAGMA user_version = 1;
             COMMIT;",
        )
        .map_err(err)?;
    }
    if version < 2 {
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE IF NOT EXISTS snippets (
                 id     INTEGER PRIMARY KEY AUTOINCREMENT,
                 phrase TEXT NOT NULL,
                 words  TEXT NOT NULL UNIQUE,
                 text   TEXT NOT NULL
             );
             PRAGMA user_version = 2;
             COMMIT;",
        )
        .map_err(err)?;
    }
    Ok(())
}

impl History {
    pub fn open(path: &Path) -> Result<Self, String> {
        // A bare file name has an empty parent, which create_dir_all would reject.
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
        let conn = Connection::open(path).map_err(err)?;
        // Wait instead of failing if another handle is briefly writing.
        conn.busy_timeout(Duration::from_secs(5)).map_err(err)?;
        // WAL lets reads proceed during a write and survives crashes well.
        // The pragma returns a row, so it must be read rather than executed.
        conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get::<_, String>(0))
            .map_err(err)?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(err)?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, String> {
        migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// A panic in another thread must not take the history down with it; the
    /// connection itself stays valid, so recover the guard.
    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn add(&self, entry: &NewEntry) -> Result<HistoryEntry, String> {
        if entry.text.trim().is_empty() {
            return Err("cannot save an empty dictation".to_string());
        }
        let created_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)
            .and_then(|d| i64::try_from(d.as_millis()).map_err(err))?;
        let conn = self.lock();
        conn.execute(
            "INSERT INTO dictations (text, app_name, created_at_ms, word_count, speech_seconds, pasted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.text,
                entry.app_name,
                created_at_ms,
                entry.word_count,
                entry.speech_seconds,
                i64::from(entry.pasted)
            ],
        )
        .map_err(err)?;
        Ok(HistoryEntry {
            id: conn.last_insert_rowid(),
            text: entry.text.to_string(),
            app_name: entry.app_name.to_string(),
            created_at_ms,
            word_count: entry.word_count,
            speech_seconds: entry.speech_seconds,
            pasted: entry.pasted,
        })
    }

    pub fn recent(&self, limit: usize, before_id: Option<i64>) -> Result<Vec<HistoryEntry>, String> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare_cached(&format!(
                "SELECT {COLUMNS} FROM dictations
                 WHERE (?1 IS NULL OR id < ?1)
                 ORDER BY id DESC LIMIT ?2"
            ))
            .map_err(err)?;
        let rows = stmt
            .query_map(params![before_id, sql_limit(limit)], row_to_entry)
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    /// Finds dictations by reading each one, for text the word index can't split into words.
    fn search_anywhere(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, String> {
        let patterns: Vec<String> = query.split_whitespace().map(anywhere_pattern).collect();
        if patterns.is_empty() {
            return Ok(Vec::new());
        }
        let conditions = vec!["text LIKE ? ESCAPE '\\'"; patterns.len()].join(" AND ");
        let conn = self.lock();
        let mut stmt = conn
            .prepare_cached(&format!("SELECT {COLUMNS} FROM dictations WHERE {conditions} ORDER BY id DESC LIMIT {}", sql_limit(limit)))
            .map_err(err)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(patterns.iter()), row_to_entry).map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, String> {
        if has_unspaced_language(query) {
            return self.search_anywhere(query, limit);
        }
        let Some(fts_query) = build_fts_query(query) else {
            return Ok(Vec::new());
        };
        let conn = self.lock();
        let mut stmt = conn
            .prepare_cached(&format!(
                "SELECT {COLUMNS} FROM dictations
                 WHERE id IN (SELECT rowid FROM dictations_fts WHERE dictations_fts MATCH ?1)
                 ORDER BY id DESC LIMIT ?2"
            ))
            .map_err(err)?;
        let rows = stmt
            .query_map(params![fts_query, sql_limit(limit)], row_to_entry)
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    pub fn get(&self, id: i64) -> Result<Option<HistoryEntry>, String> {
        let conn = self.lock();
        conn.query_row(
            &format!("SELECT {COLUMNS} FROM dictations WHERE id = ?1"),
            params![id],
            row_to_entry,
        )
        .optional()
        .map_err(err)
    }

    pub fn delete(&self, id: i64) -> Result<bool, String> {
        let conn = self.lock();
        let removed = conn
            .execute("DELETE FROM dictations WHERE id = ?1", params![id])
            .map_err(err)?;
        Ok(removed > 0)
    }

    /// Deletes every dictation, then rewrites the file so the old words aren't left in its free space.
    /// Snippets stay.
    pub fn clear(&self) -> Result<usize, String> {
        let conn = self.lock();
        let removed = conn.execute("DELETE FROM dictations", []).map_err(err)?;
        conn.execute_batch(
            "INSERT INTO dictations_fts(dictations_fts) VALUES('rebuild');
             VACUUM;",
        )
        .map_err(err)?;
        // A file database also keeps recent changes in a side file until it's folded back in.
        let _ = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
        Ok(removed)
    }

    pub fn count(&self) -> Result<i64, String> {
        let conn = self.lock();
        conn.query_row("SELECT COUNT(*) FROM dictations", [], |r| r.get(0))
            .map_err(err)
    }

    pub fn snippets(&self) -> Result<Vec<Snippet>, String> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare_cached("SELECT id, phrase, text FROM snippets ORDER BY phrase COLLATE NOCASE")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Snippet { id: row.get(0)?, phrase: row.get(1)?, text: row.get(2)? })
            })
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    /// Adds a snippet, or changes the one with this id. Two snippets can't share a phrase.
    pub fn save_snippet(&self, id: Option<i64>, phrase: &str, text: &str) -> Result<Snippet, String> {
        if let Some(problem) = snippets::problem_with(phrase, text) {
            return Err(problem);
        }
        let phrase = phrase.trim();
        let words = snippets::phrase_words(phrase).join(" ");
        let conn = self.lock();
        let taken: Option<i64> = conn
            .query_row("SELECT id FROM snippets WHERE words = ?1", params![words], |row| row.get(0))
            .optional()
            .map_err(err)?;
        if taken.is_some_and(|other| Some(other) != id) {
            return Err("You already have a snippet with that phrase.".to_string());
        }
        let id = match id {
            Some(id) => {
                let changed = conn
                    .execute(
                        "UPDATE snippets SET phrase = ?1, words = ?2, text = ?3 WHERE id = ?4",
                        params![phrase, words, text, id],
                    )
                    .map_err(err)?;
                if changed == 0 {
                    return Err("That snippet no longer exists.".to_string());
                }
                id
            }
            None => {
                conn.execute(
                    "INSERT INTO snippets (phrase, words, text) VALUES (?1, ?2, ?3)",
                    params![phrase, words, text],
                )
                .map_err(err)?;
                conn.last_insert_rowid()
            }
        };
        Ok(Snippet { id, phrase: phrase.to_string(), text: text.to_string() })
    }

    pub fn delete_snippet(&self, id: i64) -> Result<bool, String> {
        let conn = self.lock();
        let removed = conn.execute("DELETE FROM snippets WHERE id = ?1", params![id]).map_err(err)?;
        Ok(removed > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Instant;

    fn new(text: &str) -> NewEntry<'_> {
        NewEntry {
            text,
            app_name: "Notepad",
            word_count: 3,
            speech_seconds: 1.5,
            pasted: true,
        }
    }

    fn mem() -> History {
        History::open_in_memory().expect("open in memory")
    }

    fn texts(v: &[HistoryEntry]) -> Vec<&str> {
        v.iter().map(|e| e.text.as_str()).collect()
    }

    /// Temp folder removed on drop, even if the test fails.
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let p = std::env::temp_dir().join(format!(
                "mimi-history-test-{tag}-{}-{nanos}",
                std::process::id()
            ));
            TempDir(p)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn add_and_read_back_all_fields() {
        let h = mem();
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let added = h
            .add(&NewEntry {
                text: "hello world",
                app_name: "Code",
                word_count: 2,
                speech_seconds: 0.75,
                pasted: false,
            })
            .unwrap();
        assert!(added.id > 0);
        assert!(added.created_at_ms >= before);
        let got = h.get(added.id).unwrap().unwrap();
        assert_eq!(got, added);
        assert_eq!(got.text, "hello world");
        assert_eq!(got.app_name, "Code");
        assert_eq!(got.word_count, 2);
        assert_eq!(got.speech_seconds, 0.75);
        assert!(!got.pasted);
        assert!(h.get(added.id + 100).unwrap().is_none());
    }

    #[test]
    fn blank_text_is_refused() {
        let h = mem();
        assert!(h.add(&new("")).is_err());
        assert!(h.add(&new("   \t\n ")).is_err());
        assert_eq!(h.count().unwrap(), 0);
    }

    #[test]
    fn recent_is_newest_first_and_limited() {
        let h = mem();
        for t in ["one", "two", "three", "four"] {
            h.add(&new(t)).unwrap();
        }
        assert_eq!(texts(&h.recent(10, None).unwrap()), ["four", "three", "two", "one"]);
        assert_eq!(texts(&h.recent(2, None).unwrap()), ["four", "three"]);
        assert!(h.recent(0, None).unwrap().is_empty());
    }

    #[test]
    fn paging_with_before_id() {
        let h = mem();
        for i in 1..=5 {
            h.add(&new(&format!("item {i}"))).unwrap();
        }
        let page1 = h.recent(2, None).unwrap();
        assert_eq!(texts(&page1), ["item 5", "item 4"]);
        let page2 = h.recent(2, Some(page1[1].id)).unwrap();
        assert_eq!(texts(&page2), ["item 3", "item 2"]);
        let page3 = h.recent(2, Some(page2[1].id)).unwrap();
        assert_eq!(texts(&page3), ["item 1"]);
        assert!(h.recent(2, Some(page3[0].id)).unwrap().is_empty());
    }

    #[test]
    fn search_ignores_accents_both_ways() {
        let h = mem();
        h.add(&new("Tengo una reunión mañana")).unwrap();
        h.add(&new("la reunion de ayer")).unwrap();
        // Unaccented query finds accented text and vice versa.
        assert_eq!(h.search("reunion", 10).unwrap().len(), 2);
        assert_eq!(h.search("reunión", 10).unwrap().len(), 2);
        assert_eq!(h.search("manana", 10).unwrap().len(), 1);
        assert_eq!(h.search("MAÑANA", 10).unwrap().len(), 1);
    }

    #[test]
    fn english_search_is_case_insensitive_and_newest_first() {
        let h = mem();
        h.add(&new("Send the report today")).unwrap();
        h.add(&new("nothing relevant")).unwrap();
        h.add(&new("The REPORT is late")).unwrap();
        let found = h.search("report", 10).unwrap();
        assert_eq!(texts(&found), ["The REPORT is late", "Send the report today"]);
        assert_eq!(h.search("report", 1).unwrap().len(), 1);
    }

    #[test]
    fn multi_word_search_requires_all_words() {
        let h = mem();
        h.add(&new("meeting with the client")).unwrap();
        h.add(&new("meeting about budget")).unwrap();
        assert_eq!(h.search("meeting client", 10).unwrap().len(), 1);
        assert_eq!(h.search("  meeting   budget ", 10).unwrap().len(), 1);
        assert_eq!(h.search("meeting", 10).unwrap().len(), 2);
        assert!(h.search("meeting zebra", 10).unwrap().is_empty());
    }

    #[test]
    fn prefix_search() {
        let h = mem();
        h.add(&new("Tengo una reunión")).unwrap();
        assert_eq!(h.search("reun", 10).unwrap().len(), 1);
        assert_eq!(h.search("tengo reu", 10).unwrap().len(), 1);
        assert!(h.search("union", 10).unwrap().is_empty());
    }

    #[test]
    fn blank_query_returns_nothing() {
        let h = mem();
        h.add(&new("something")).unwrap();
        assert!(h.search("", 10).unwrap().is_empty());
        assert!(h.search("   \t ", 10).unwrap().is_empty());
    }

    #[test]
    fn hostile_queries_do_not_error_or_damage_data() {
        let h = mem();
        h.add(&new("foo bar baz")).unwrap();
        h.add(&new("x marks the spot")).unwrap();
        let hostile = [
            "\"",
            "*",
            "foo\"bar",
            "AND",
            "OR",
            "NOT",
            "NEAR(a b)",
            "(",
            ")",
            "a:b",
            "text:foo",
            "-x",
            "^foo",
            "foo*",
            "\"\"\"",
            "'; DROP TABLE dictations; --",
            "\\",
            "a\u{0}b",
        ];
        for q in hostile {
            assert!(h.search(q, 10).is_ok(), "query {q:?} errored");
        }
        assert_eq!(h.count().unwrap(), 2);
        assert_eq!(h.search("foo", 10).unwrap().len(), 1);
        // A leading minus is literal input, not an exclusion operator.
        assert_eq!(h.search("-x", 10).unwrap().len(), 1);
        // Keywords are plain words here, not operators.
        assert!(h.search("foo AND zzz", 10).unwrap().is_empty());
    }

    #[test]
    fn delete_removes_from_listing_and_search() {
        let h = mem();
        let a = h.add(&new("alpha unique")).unwrap();
        let b = h.add(&new("beta other")).unwrap();
        assert!(h.delete(a.id).unwrap());
        assert!(!h.delete(a.id).unwrap());
        assert!(h.get(a.id).unwrap().is_none());
        assert_eq!(texts(&h.recent(10, None).unwrap()), ["beta other"]);
        assert!(h.search("alpha", 10).unwrap().is_empty());
        assert_eq!(h.search("beta", 10).unwrap()[0].id, b.id);
        assert!(!h.delete(9999).unwrap());
    }

    #[test]
    fn clearing_removes_every_dictation_from_the_file_but_keeps_snippets() {
        let dir = TempDir::new("clear");
        std::fs::create_dir_all(&dir.0).unwrap();
        let path = dir.0.join("history.db");
        let h = History::open(&path).unwrap();
        for index in 0..300 {
            h.add(&new(&format!("zanahoriasecreta número {index}"))).unwrap();
        }
        h.save_snippet(None, "mi dirección", "Calle Falsa 123").unwrap();
        assert_eq!(h.clear().unwrap(), 300);
        assert_eq!(h.count().unwrap(), 0);
        assert!(h.recent(10, None).unwrap().is_empty());
        assert!(h.search("zanahoriasecreta", 10).unwrap().is_empty());
        assert_eq!(h.snippets().unwrap().len(), 1);
        for name in ["history.db", "history.db-wal"] {
            if let Ok(bytes) = std::fs::read(dir.0.join(name)) {
                let found = bytes.windows(16).any(|window| window == b"zanahoriasecreta");
                assert!(!found, "old words left in {name}");
            }
        }
        let added = h.add(&new("after clearing")).unwrap();
        assert_eq!(h.search("clearing", 10).unwrap()[0].id, added.id);
        assert_eq!(mem().clear().unwrap(), 0);
    }

    #[test]
    fn count_tracks_adds_and_deletes() {
        let h = mem();
        assert_eq!(h.count().unwrap(), 0);
        let a = h.add(&new("a")).unwrap();
        h.add(&new("b")).unwrap();
        assert_eq!(h.count().unwrap(), 2);
        h.delete(a.id).unwrap();
        assert_eq!(h.count().unwrap(), 1);
    }

    #[test]
    fn data_survives_reopening_a_file_database() {
        let dir = TempDir::new("reopen");
        // A nested path checks that parent folders are created.
        let path = dir.0.join("nested").join("history.db");
        let id = {
            let h = History::open(&path).unwrap();
            h.add(&new("persistente reunión")).unwrap().id
        };
        let h = History::open(&path).unwrap();
        assert_eq!(h.count().unwrap(), 1);
        assert_eq!(h.get(id).unwrap().unwrap().text, "persistente reunión");
        // The index must also survive, not just the table.
        assert_eq!(h.search("reunion", 10).unwrap().len(), 1);
        let version: i64 = h
            .lock()
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let mode: String = h
            .lock()
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
    }

    #[test]
    fn snippets_can_be_added_changed_and_deleted() {
        let h = mem();
        let added = h.save_snippet(None, " Mi Dirección ", "Calle Luna 5").unwrap();
        assert_eq!(added.phrase, "Mi Dirección");
        let changed = h.save_snippet(Some(added.id), "mi direccion", "Calle Sol 9").unwrap();
        assert_eq!(h.snippets().unwrap(), vec![changed]);
        assert!(h.delete_snippet(added.id).unwrap());
        assert!(!h.delete_snippet(added.id).unwrap());
        assert!(h.snippets().unwrap().is_empty());
    }

    #[test]
    fn two_snippets_cannot_share_a_phrase() {
        let h = mem();
        h.save_snippet(None, "mi dirección", "uno").unwrap();
        let second = h.save_snippet(None, "other phrase", "dos").unwrap();
        assert!(h.save_snippet(None, "MI DIRECCION.", "tres").is_err());
        assert!(h.save_snippet(Some(second.id), "mi dirección", "tres").is_err());
        assert_eq!(h.snippets().unwrap().len(), 2);
    }

    #[test]
    fn a_risky_snippet_is_refused_and_a_missing_one_reports_it() {
        let h = mem();
        assert!(h.save_snippet(None, "dirección", "uno").is_err());
        assert!(h.save_snippet(None, "nueva línea", "uno").is_err());
        assert!(h.save_snippet(Some(99), "mi dirección", "uno").is_err());
        assert!(h.snippets().unwrap().is_empty());
    }

    #[test]
    fn a_version_one_history_gains_snippets_and_keeps_its_dictations() {
        let dir = TempDir::new("upgrade");
        std::fs::create_dir_all(&dir.0).unwrap();
        let path = dir.0.join("history.db");
        {
            let h = History::open(&path).unwrap();
            h.add(&new("dictado anterior")).unwrap();
            h.lock().execute_batch("DROP TABLE snippets; PRAGMA user_version = 1;").unwrap();
        }
        let h = History::open(&path).unwrap();
        assert_eq!(h.count().unwrap(), 1);
        h.save_snippet(None, "mi dirección", "Calle Luna 5").unwrap();
        assert_eq!(h.snippets().unwrap().len(), 1);
    }

    #[test]
    fn japanese_text_round_trips() {
        let h = mem();
        let text = "今日は会議があります。よろしくお願いします";
        let added = h.add(&new(text)).unwrap();
        assert_eq!(h.get(added.id).unwrap().unwrap().text, text);
        assert_eq!(h.recent(1, None).unwrap()[0].text, text);
        assert_eq!(h.search("今日", 10).unwrap().len(), 1);
    }

    #[test]
    fn japanese_search_finds_text_in_the_middle() {
        let h = mem();
        h.add(&new("今日は予約がたくさんあって疲れました。")).unwrap();
        h.add(&new("明日の会議は十時からです。")).unwrap();
        h.add(&new("予約 mixed with English")).unwrap();
        assert_eq!(h.search("予約", 10).unwrap().len(), 2);
        assert_eq!(h.search("たくさん", 10).unwrap().len(), 1);
        assert_eq!(h.search("十時", 10).unwrap().len(), 1);
        assert_eq!(h.search("予約 mixed", 10).unwrap().len(), 1);
        assert!(h.search("東京", 10).unwrap().is_empty());
    }

    #[test]
    fn wildcards_in_a_japanese_search_are_plain_characters() {
        let h = mem();
        h.add(&new("予約 100% 完了")).unwrap();
        h.add(&new("予約 済み")).unwrap();
        assert_eq!(h.search("予約 100%", 10).unwrap().len(), 1);
        assert!(h.search("予約 %_", 10).unwrap().is_empty());
        assert!(h.search("予約 \\", 10).unwrap().is_empty());
    }

    #[test]
    fn many_inserts_and_search_stay_fast() {
        let h = mem();
        let start = Instant::now();
        {
            // One transaction keeps the bulk load quick; the history API is unchanged.
            let conn = h.lock();
            conn.execute_batch("BEGIN").unwrap();
        }
        for i in 0..10_000 {
            h.add(&new(&format!("dictado número {i} sobre la reunión de equipo"))).unwrap();
        }
        h.lock().execute_batch("COMMIT").unwrap();
        let insert_time = start.elapsed();
        assert_eq!(h.count().unwrap(), 10_000);

        let t = Instant::now();
        let found = h.search("reunion equ", 50).unwrap();
        let search_time = t.elapsed();
        assert_eq!(found.len(), 50);
        assert!(found[0].id > found[1].id);

        let (insert_cap, search_cap) = if cfg!(debug_assertions) {
            (Duration::from_secs(60), Duration::from_millis(1000))
        } else {
            (Duration::from_secs(10), Duration::from_millis(50))
        };
        assert!(insert_time < insert_cap, "inserts took {insert_time:?}");
        assert!(search_time < search_cap, "search took {search_time:?}");
    }
}
