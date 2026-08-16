use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

pub struct ClipboardEntry {
    pub id: i32,
    pub content: String,
    pub timestamp: String,
}

pub struct ClipboardDatabase {
    conn: Connection,
}

impl ClipboardDatabase {
    pub fn init(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;

        // WAL lets the clipboard monitor thread write while the search UI
        // reads, without locking each other out. NORMAL is durable enough for
        // a clipboard cache and avoids an fsync on every insert.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        // Give a colliding reader/writer 5s to release the lock instead of
        // failing with SQLITE_BUSY.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn add_entry(&self, content: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO clipboard_history (content) VALUES (?1)",
            params![content],
        )?;

        // Prune old entries to keep the table fast (keep last 200).
        self.conn.execute(
            "DELETE FROM clipboard_history WHERE id NOT IN (
                SELECT id FROM clipboard_history ORDER BY id DESC LIMIT 200
            )",
            [],
        )?;

        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<Vec<ClipboardEntry>> {
        // Escape LIKE wildcards so user input is matched literally.
        let escaped = query.replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{}%", escaped);

        // prepare_cached reuses the compiled statement across keystrokes
        // instead of re-parsing SQL on every search.
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, content, timestamp FROM clipboard_history \
             WHERE content LIKE ?1 ESCAPE '\\' ORDER BY id DESC LIMIT 50",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(ClipboardEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn get_entry(&self, id: i32) -> Result<String> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT content FROM clipboard_history WHERE id = ?1")?;
        stmt.query_row(params![id], |row| row.get(0))
    }

    pub fn clear_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM clipboard_history", [])?;
        Ok(())
    }
}
