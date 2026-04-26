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
            "INSERT INTO clipboard_history (content) VALUES (?)",
            params![content],
        )?;
        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<Vec<ClipboardEntry>> {
        let mut stmt = self.conn.prepare("SELECT id, content, timestamp FROM clipboard_history WHERE content LIKE ? ORDER BY id DESC LIMIT 50")?;
        let rows = stmt.query_map(params![format!("%{}%", query)], |row| {
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
        let mut stmt = self.conn.prepare("SELECT content FROM clipboard_history WHERE id = ?")?;
        stmt.query_row(params![id], |row| row.get(0))
    }

    pub fn clear_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM clipboard_history", [])?;
        Ok(())
    }
}
