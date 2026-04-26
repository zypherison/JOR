use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use crate::plugins::clipboard_db::ClipboardDatabase;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use arboard::Clipboard;
use tauri::Manager;

pub struct ClipboardPlugin {
    db: Arc<Mutex<Option<ClipboardDatabase>>>,
}

impl ClipboardPlugin {
    pub fn new() -> Self {
        Self {
            db: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl Plugin for ClipboardPlugin {
    fn id(&self) -> &str { "clipboard" }
    fn name(&self) -> &str { "Clipboard Manager" }

    fn init(&self, app: &tauri::AppHandle) {
        let app_dir = app.path().app_data_dir().unwrap_or_else(|_| std::env::current_dir().unwrap());
        let db_path = app_dir.join("clipboard.db");
        
        // Ensure directory exists
        if !app_dir.exists() {
            let _ = std::fs::create_dir_all(&app_dir);
        }

        match ClipboardDatabase::init(db_path) {
            Ok(db) => {
                let mut db_guard = self.db.lock().unwrap();
                *db_guard = Some(db);
                
                let monitor_db_arc = self.db.clone();
                thread::spawn(move || {
                    let mut last_content = String::new();
                    loop {
                        if let Ok(mut clipboard) = Clipboard::new() {
                            loop {
                                match clipboard.get_text() {
                                    Ok(content) => {
                                        if !content.is_empty() && content != last_content {
                                            if let Ok(db_lock) = monitor_db_arc.lock() {
                                                if let Some(db) = db_lock.as_ref() {
                                                    if db.add_entry(&content).is_ok() {
                                                        last_content = content.clone();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        // On any error (locked, permission, etc.), break to re-init
                                        break;
                                    }
                                }
                                thread::sleep(Duration::from_millis(500));
                            }
                        }
                        // Wait before retrying initialization
                        thread::sleep(Duration::from_secs(2));
                    }
                });
            }
            Err(e) => {
                eprintln!("Failed to initialize clipboard database: {}", e);
            }
        }
    }

    async fn search(&self, query: &str, mode: &str) -> Vec<Entry> {
        let mut results = Vec::new();
        if let Ok(db_guard) = self.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                // In standard mode, don't show clipboard for empty queries
                if mode == "standard" && query.is_empty() {
                    return vec![];
                }

                let search_results = if query.is_empty() {
                    db.search("").ok().unwrap_or_default()
                } else {
                    db.search(query).ok().unwrap_or_default()
                };

                for (i, entry) in search_results.into_iter().enumerate() {
                    // In clipboard mode, score high (200). In standard mode, score low (40).
                    let base_score = if mode == "clipboard" { 200 } else { 40 };
                    let score = base_score - (i as i64);

                    results.push(Entry {
                        name: entry.content.chars().take(80).collect::<String>().replace('\n', " ").trim().to_string(),
                        name_lower: entry.content.to_lowercase(),
                        path: format!("clipboard:{}", entry.id),
                        subtitle: format!("Clipboard • {}", entry.timestamp),
                        kind: EntryKind::Plugin,
                        score: 90,
                        search_score: score,
                    });
                }
            }
        }
        results
    }

    async fn execute(&self, action_id: &str) -> Result<(), String> {
        if action_id == "clear_all" {
            if let Ok(db_guard) = self.db.lock() {
                if let Some(db) = db_guard.as_ref() {
                    db.clear_all().map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
        }

        if let Ok(id) = action_id.parse::<i32>() {
            if let Ok(db_guard) = self.db.lock() {
                if let Some(db) = db_guard.as_ref() {
                    if let Ok(content) = db.get_entry(id) {
                        if let Ok(mut clipboard) = Clipboard::new() {
                            clipboard.set_text(content).map_err(|e| e.to_string())?;
                            return Ok(());
                        }
                    }
                }
            }
        }
        Err("Failed to copy from history".into())
    }
}
