// ─────────────────────────────────────────────────────────────
// JOR — File System Indexer
// Crawls standard Windows directories and builds a searchable
// in-memory index of apps, files, and folders.
// ─────────────────────────────────────────────────────────────

use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::models::{Entry, EntryKind};
use std::io;
use std::collections::HashSet;

pub struct Indexer;

// ── Extension categories ────────────────────────────────────
const EXT_APP:     &[&str] = &["lnk", "exe", "bat", "cmd"];
const EXT_DOC:     &[&str] = &["pdf", "docx", "doc", "xlsx", "xls", "csv", "pptx", "ppt", "txt", "md", "rtf", "odt", "log", "ini", "cfg", "conf"];
const EXT_IMAGE:   &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "svg", "webp", "ico"];
const EXT_VIDEO:   &[&str] = &["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"];
const EXT_AUDIO:   &[&str] = &["mp3", "wav", "flac", "aac", "ogg", "m4a"];
const EXT_ARCHIVE: &[&str] = &["zip", "rar", "7z", "tar", "gz"];
const EXT_CODE:    &[&str] = &["rs", "js", "ts", "py", "go", "java", "c", "cpp", "html", "css", "json", "yaml", "toml", "xml", "sh", "ps1", "sql", "sqlite", "db"];

impl Indexer {

    /// Build the complete index by mapping the entire application ecosystem
    /// and core user directories.
    pub fn index_all(extra_paths: &[String]) -> Vec<Entry> {
        let mut entries = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut paths_to_index: Vec<(PathBuf, usize)> = Vec::new();

        // ── 1. THE APPLICATION ARTERIES (Deep Crawl) ─────────
        // These are where 99.9% of Windows apps live.
        if let Some(mut p) = dirs::data_dir() {
            p.push("Microsoft\\Windows\\Start Menu\\Programs");
            paths_to_index.push((p, 8)); // Deep scan for folders
        }
        paths_to_index.push((PathBuf::from("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs"), 8));
        
        if let Some(mut p) = dirs::home_dir() {
            p.push("AppData\\Local\\Programs");
            if p.exists() { paths_to_index.push((p, 4)); }
        }

        // ── 2. SYSTEM BINARIES (Shallow but Wide) ────────────
        paths_to_index.push((PathBuf::from("C:\\Program Files"), 3));
        paths_to_index.push((PathBuf::from("C:\\Program Files (x86)"), 3));

        // ── 3. USER HUB (Shallow Crawl) ──────────────────────
        // We want the files and folders here, but not to crawl their internals.
        if let Some(p) = dirs::desktop_dir()  { paths_to_index.push((p, 2)); }
        if let Some(p) = dirs::document_dir() { paths_to_index.push((p, 2)); }
        if let Some(p) = dirs::download_dir() { paths_to_index.push((p, 2)); }
        if let Some(p) = dirs::picture_dir()  { paths_to_index.push((p, 2)); }
        if let Some(p) = dirs::video_dir()    { paths_to_index.push((p, 2)); }
        if let Some(p) = dirs::audio_dir()    { paths_to_index.push((p, 2)); }

        for extra in extra_paths {
            let p = PathBuf::from(extra);
            if p.exists() { paths_to_index.push((p, 2)); }
        }

        // ── EXECUTION ───────────────────────────────────────
        for (root, max_depth) in paths_to_index {
            if !root.exists() { continue; }

            for entry in WalkDir::new(&root)
                .max_depth(max_depth)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                let path_str = path.to_string_lossy().to_string();
                if visited.contains(&path_str) { continue; }
                
                // Smart Skip: Ignore common massive folders and hidden junk
                let name_lower = entry.file_name().to_string_lossy().to_lowercase();
                if name_lower.starts_with('.') || name_lower.starts_with('$') ||
                   path_str.contains("node_modules") || path_str.contains(".git") ||
                   path_str.contains("target") || path_str.contains("cache") ||
                   path_str.contains("Logs") || path_str.contains("Locales")
                {
                   continue;
                }

                visited.insert(path_str.clone());
                if let Some(entry_obj) = Self::process_path(path) {
                    entries.push(entry_obj);
                }
            }
        }

        // ── 4. SYSTEM POWER ACTIONS ─────────────────────────
        let system_actions = vec![
            ("Sleep",     "System Action", "rundll32.exe powrprof.dll,SetSuspendState 0,1,0"),
            ("Shut Down", "System Action", "shutdown /s /t 0"),
            ("Restart",   "System Action", "shutdown /r /t 0"),
            ("Lock Screen", "System Action", "rundll32.exe user32.dll,LockWorkStation"),
        ];
        for (name, sub, cmd) in system_actions {
            entries.push(Entry {
                name: name.to_string(),
                name_lower: name.to_lowercase(),
                path: cmd.to_string(),
                subtitle: sub.to_string(),
                kind: EntryKind::System,
                score: 100,
                accessories: Some(vec!["SYS".to_string()]),
                keywords: None,
            });
        }
        entries
    }

    fn process_path(path: &Path) -> Option<Entry> {
        let name_os = path.file_name()?;
        let name_str = name_os.to_str()?;
        let path_str = path.to_string_lossy().to_string();
        let name_lower = name_str.to_lowercase();

        // ── 1. GLOBAL JUNK FILTER ───────────────────────────
        // Skip hidden files and common system noise
        if name_str.starts_with('.') || name_str.starts_with('$') { return None; }

        // Blacklist common "non-app" executables and background tools
        let junk_exes = ["python", "pythonw", "cmd", "conhost", "vc_redist", "vcredist", 
                         "unitycrashhandler", "crashpad", "helper", "elevated", "setup", 
                         "uninstall", "install", "update", "patch", "repair"];
        
        for junk in &junk_exes {
            if name_lower.contains(junk) { return None; }
        }

        // ── 2. APPLICATION DETECTION ───────────────────────
        if !path.is_dir() {
            let extension = path.extension()?.to_str()?.to_lowercase();
            
            if EXT_APP.contains(&extension.as_str()) {
                // Ignore executables in deep internal resource folders
                let internal_dirs = ["node_modules", "site-packages", "resources", "locales", 
                                     "bin", "lib", "dist", "target", "cache", "tmp", "temp"];
                for dir in &internal_dirs {
                    if path_str.to_lowercase().contains(&format!("\\{}\\ ", dir).replace(" ", "")) {
                        // Special exception: allow bin/ if it's very shallow in the path? 
                        // No, for now let's be strict to keep it clean.
                        return None;
                    }
                }

                let stem = path.file_stem()?.to_str()?.to_string();
                return Some(Entry {
                    name: stem.clone(),
                    name_lower: stem.to_lowercase(),
                    path: path_str,
                    subtitle: "Application".to_string(),
                    kind: EntryKind::App,
                    score: 250,
                    accessories: Some(vec!["EXE".to_string()]),
                    keywords: None,
                });
            }

            // Document & Media Types
            if EXT_DOC.contains(&extension.as_str()) || EXT_IMAGE.contains(&extension.as_str()) ||
               EXT_VIDEO.contains(&extension.as_str()) || EXT_AUDIO.contains(&extension.as_str()) ||
               EXT_CODE.contains(&extension.as_str()) {
                
                return Some(Entry {
                    name: name_str.to_string(),
                    name_lower: name_lower,
                    path: path_str,
                    subtitle: format!("{} File", extension.to_uppercase()),
                    kind: EntryKind::File,
                    score: 0,
                    accessories: Some(vec![extension.to_uppercase()]),
                    keywords: None,
                });
            }
        } else {
            // ── FOLDER DETECTION ──────────────────────────────
            // We want to be able to open top-level folders.
            let parent = path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Folder")
                .to_string();

            return Some(Entry {
                name: name_str.to_string(),
                name_lower: name_lower,
                path: path_str,
                subtitle: parent,
                kind: EntryKind::Folder,
                score: 50,
                accessories: Some(vec!["DIR".to_string()]),
                keywords: None,
            });
        }
        None
    }

    /// Serialize index to disk for fast startup.
    pub fn save_index(entries: &[Entry], path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let encoded = bincode::serialize(entries)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, encoded)?;
        Ok(())
    }

    /// Deserialize a cached index from disk.
    pub fn load_index(path: &Path) -> std::io::Result<Vec<Entry>> {
        let encoded = fs::read(path)?;
        let entries = bincode::deserialize(&encoded)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(entries)
    }
}
