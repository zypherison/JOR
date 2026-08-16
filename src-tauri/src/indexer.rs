// ─────────────────────────────────────────────────────────────
// JOR — File System Indexer
// Crawls the platform's standard directories (Start Menu / .desktop
// applications, user folders, extra paths) and builds a searchable
// in-memory index of apps, files, and folders.
// ─────────────────────────────────────────────────────────────

use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::models::{Entry, EntryKind};
use std::io::{Read, Write};
use std::collections::HashSet;
use tauri::Manager;

pub struct Indexer;

// ── Extension categories ────────────────────────────────────
#[cfg(target_os = "windows")]
const EXT_APP:     &[&str] = &["lnk", "exe"];
#[cfg(not(target_os = "windows"))]
const EXT_APP:     &[&str] = &["desktop", "appimage"];
const EXT_DOC:     &[&str] = &["pdf", "docx", "doc", "xlsx", "xls", "csv", "pptx", "ppt", "txt", "md", "rtf", "odt"];
const EXT_IMAGE:   &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "svg", "webp", "ico"];
const EXT_VIDEO:   &[&str] = &["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"];
const EXT_AUDIO:   &[&str] = &["mp3", "wav", "flac", "aac", "ogg", "m4a"];
const EXT_ARCHIVE: &[&str] = &["zip", "rar", "7z", "tar", "gz"];
const EXT_CODE:    &[&str] = &["rs", "js", "ts", "py", "go", "java", "c", "cpp", "html", "css", "json", "yaml", "toml", "xml", "sh", "bat", "cmd", "ps1"];

impl Indexer {
    /// Build the complete index from all configured directories.
    /// Uses a cache file for instant startup if available.
    pub fn index_all(app: &tauri::AppHandle, extra_paths: &[String]) -> Vec<Entry> {
        let cache_path = app.path().app_cache_dir().unwrap_or_default().join("index.bin");
        
        // Try to load from cache first for speed
        if let Ok(cached) = Self::load_index(&cache_path) {
            return cached;
        }

        let entries = Self::perform_full_index(extra_paths);
        let _ = Self::save_index(&entries, &cache_path);
        entries
    }

    pub fn refresh_index(app: &tauri::AppHandle, extra_paths: Vec<String>) {
        let ah = app.clone();
        let cache_path = app.path().app_cache_dir().unwrap_or_default().join("index.bin");
        
        tauri::async_runtime::spawn(async move {
            let fresh = Self::perform_full_index(&extra_paths);
            let _ = Self::save_index(&fresh, &cache_path);
            
            // Update the running state, preserving workflow entries — they are
            // injected separately by main.rs and are not part of the file index.
            if let Some(state) = ah.try_state::<crate::AppState>() {
                if let Ok(mut entries) = state.entries.lock() {
                    entries.retain(|e| e.kind == EntryKind::Workflow);
                    entries.extend(fresh);
                }
            }
        });
    }

    fn perform_full_index(extra_paths: &[String]) -> Vec<Entry> {
        let mut entries = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut paths_to_index: Vec<(PathBuf, bool)> = Vec::new();

        // ── App shortcuts (deep crawl) ──────────────────────
        #[cfg(target_os = "windows")]
        {
            if let Some(mut p) = dirs::data_dir() {
                p.push("Microsoft\\Windows\\Start Menu\\Programs");
                paths_to_index.push((p, true));
            }
            paths_to_index.push((
                PathBuf::from("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs"), true
            ));
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(mut p) = dirs::data_dir() {
                p.push("applications");
                paths_to_index.push((p, true));
            }
            paths_to_index.push((PathBuf::from("/usr/share/applications"), true));
            paths_to_index.push((PathBuf::from("/var/lib/flatpak/exports/share/applications"), true));
        }

        // ── User directories (shallow — top 2 levels) ───────
        if let Some(p) = dirs::desktop_dir()  { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::document_dir() { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::download_dir() { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::picture_dir()  { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::video_dir()    { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::audio_dir()    { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::home_dir()     { paths_to_index.push((p, false)); }
        
        // ── Program Files (shallow, Windows only) ───────────
        #[cfg(target_os = "windows")]
        {
            paths_to_index.push((PathBuf::from("C:\\Program Files"), false));
            paths_to_index.push((PathBuf::from("C:\\Program Files (x86)"), false));
        }

        for extra in extra_paths {
            let p = PathBuf::from(extra);
            if p.exists() { paths_to_index.push((p, false)); }
        }

        for (root, deep) in &paths_to_index {
            if !root.exists() { continue; }
            let depth = if *deep { 8 } else { 2 };

            for entry in WalkDir::new(root)
                .max_depth(depth)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                let path_str = path.to_string_lossy().to_string();
                if visited.contains(&path_str) { continue; }
                visited.insert(path_str.clone());

                if let Some(entry_obj) = Self::process_path(path) {
                    entries.push(entry_obj);
                }
            }
        }

        // Power actions differ per platform. These commands must stay in sync
        // with the allow-list in main.rs (`allowed_system_cmds`).
        #[cfg(target_os = "windows")]
        let system_actions = vec![
            ("Sleep",             "Sleep your PC",    "rundll32.exe powrprof.dll,SetSuspendState 0,1,0"),
            ("Shut Down",         "Power off",        "shutdown /s /t 0"),
            ("Restart",           "Reboot system",    "shutdown /r /t 0"),
        ];
        #[cfg(not(target_os = "windows"))]
        let system_actions = vec![
            ("Sleep",             "Suspend the system", "systemctl suspend"),
            ("Shut Down",         "Power off",          "systemctl poweroff"),
            ("Restart",           "Reboot system",      "systemctl reboot"),
        ];

        for (name, subtitle, cmd) in system_actions {
            entries.push(Entry {
                name: name.to_string(),
                name_lower: name.to_lowercase(),
                path: cmd.to_string(),
                subtitle: subtitle.to_string(),
                kind: EntryKind::System,
                score: 80,
                search_score: 0,
            });
        }

        entries
    }

    /// Classify a single filesystem path into an Entry.
    fn process_path(path: &Path) -> Option<Entry> {
        let name_os = path.file_name()?;
        let name_str = name_os.to_str()?;

        // Skip hidden / system items
        if name_str.starts_with('.') || name_str.starts_with('$') { return None; }

        // ── Directories ─────────────────────────────────────
        if path.is_dir() {
            let parent = path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            return Some(Entry {
                name: name_str.to_string(),
                name_lower: name_str.to_lowercase(),
                path: path.to_string_lossy().to_string(),
                subtitle: parent,
                kind: EntryKind::Folder,
                score: 0,
                search_score: 0,
            });
        }

        // ── Files ───────────────────────────────────────────
        let extension = path.extension()?.to_str()?.to_lowercase();
        let stem = path.file_stem()?.to_str()?.to_string();
        let ext = extension.as_str();

        let kind = if EXT_APP.contains(&ext) {
            EntryKind::App
        } else if EXT_DOC.contains(&ext) || EXT_IMAGE.contains(&ext) ||
                  EXT_VIDEO.contains(&ext) || EXT_AUDIO.contains(&ext) ||
                  EXT_ARCHIVE.contains(&ext) || EXT_CODE.contains(&ext) {
            EntryKind::File
        } else {
            return None;
        };

        // For apps (.lnk/.exe/.desktop), show just the stem; for files show full name
        let display_name = if kind == EntryKind::App {
            stem.clone()
        } else {
            format!("{}.{}", stem, extension)
        };

        // Build a human-readable subtitle from the parent directory
        let parent = path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        Some(Entry {
            name: display_name.clone(),
            name_lower: display_name.to_lowercase(),
            path: path.to_string_lossy().to_string(),
            subtitle: parent,
            kind,
            score: 0,
            search_score: 0,
        })
    }

    /// Serialize index to disk for potential future caching.
    /// Skips the write when the bytes are unchanged so background refreshes
    /// don't churn the SSD on every launch.
    pub fn save_index(entries: &[Entry], path: &Path) -> std::io::Result<()> {
        let encoded = bincode::serialize(entries)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if let Ok(existing) = fs::read(path) {
            if existing == encoded {
                return Ok(());
            }
        }
        let mut file = fs::File::create(path)?;
        file.write_all(&encoded)?;
        Ok(())
    }

    /// Deserialize a cached index from disk.
    pub fn load_index(path: &Path) -> std::io::Result<Vec<Entry>> {
        let mut file = fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let entries = bincode::deserialize(&buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(entries)
    }
}
