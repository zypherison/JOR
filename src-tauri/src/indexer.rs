// ─────────────────────────────────────────────────────────────
// JOR — File System Indexer
// Crawls standard Windows directories and builds a searchable
// in-memory index of apps, files, and folders.
// ─────────────────────────────────────────────────────────────

use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::models::{Entry, EntryKind};
use std::io::{Read, Write};
use std::collections::HashSet;

pub struct Indexer;

// ── Extension categories ────────────────────────────────────
const EXT_APP:     &[&str] = &["lnk", "exe"];
const EXT_DOC:     &[&str] = &["pdf", "docx", "doc", "xlsx", "xls", "csv", "pptx", "ppt", "txt", "md", "rtf", "odt"];
const EXT_IMAGE:   &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "svg", "webp", "ico"];
const EXT_VIDEO:   &[&str] = &["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"];
const EXT_AUDIO:   &[&str] = &["mp3", "wav", "flac", "aac", "ogg", "m4a"];
const EXT_ARCHIVE: &[&str] = &["zip", "rar", "7z", "tar", "gz"];
const EXT_CODE:    &[&str] = &["rs", "js", "ts", "py", "go", "java", "c", "cpp", "html", "css", "json", "yaml", "toml", "xml", "sh", "bat", "cmd", "ps1"];

impl Indexer {
    /// Build the complete index from all configured directories.
    /// `extra_paths` comes from the user config file.
    pub fn index_all(extra_paths: &[String]) -> Vec<Entry> {
        let mut entries = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        // (path, deep_crawl)
        let mut paths_to_index: Vec<(PathBuf, bool)> = Vec::new();

        // ── Start Menu shortcuts (deep crawl) ───────────────
        if let Some(mut p) = dirs::data_dir() {
            p.push("Microsoft\\Windows\\Start Menu\\Programs");
            paths_to_index.push((p, true));
        }
        paths_to_index.push((
            PathBuf::from("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs"), true
        ));

        // ── User directories (shallow — top 2 levels) ───────
        if let Some(p) = dirs::desktop_dir()  { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::document_dir() { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::download_dir() { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::picture_dir()  { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::video_dir()    { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::audio_dir()    { paths_to_index.push((p, false)); }
        if let Some(p) = dirs::home_dir()     { paths_to_index.push((p, false)); }

        // ── Extra user-configured paths ─────────────────────
        for extra in extra_paths {
            let p = PathBuf::from(extra);
            if p.exists() {
                paths_to_index.push((p, false));
            }
        }

        // ── Walk and process ────────────────────────────────
        for (root, deep) in &paths_to_index {
            if !root.exists() { continue; }
            let depth = if *deep { 5 } else { 2 };

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

        // ── Baked-in System Actions ─────────────────────────
        let system_actions = vec![
            ("Sleep",             "Sleep your PC",    "rundll32.exe powrprof.dll,SetSuspendState 0,1,0"),
            ("Shut Down",         "Power off",        "shutdown /s /t 0"),
            ("Restart",           "Reboot system",    "shutdown /r /t 0"),
        ];

        for (name, subtitle, cmd) in system_actions {
            entries.push(Entry {
                name: name.to_string(),
                name_lower: name.to_lowercase(),
                path: cmd.to_string(),
                subtitle: subtitle.to_string(),
                kind: EntryKind::System,
                score: 80,
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

        // For apps (.lnk/.exe), show just the stem; for files show full name
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
        })
    }

    /// Serialize index to disk for potential future caching.
    pub fn save_index(entries: &[Entry], path: &Path) -> std::io::Result<()> {
        let encoded = bincode::serialize(entries)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let mut file = fs::File::create(path)?;
        file.write_all(&encoded)?;
        Ok(())
    }

    /// Deserialize a cached index from disk.
    #[allow(dead_code)]
    pub fn load_index(path: &Path) -> std::io::Result<Vec<Entry>> {
        let mut file = fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let entries = bincode::deserialize(&buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(entries)
    }
}
