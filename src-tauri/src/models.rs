// ─────────────────────────────────────────────────────────────
// JOR — Models & Type Definitions
// Defines all core data structures used across the launcher.
// ─────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

/// Classification of an indexed entry. Each variant maps to an
/// integer for compact serialization and frontend icon mapping.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum EntryKind {
    App      = 0,
    File     = 1,
    Folder   = 2,
    System   = 3,
    Web      = 4,
    Math     = 5,
    Workflow = 6,
    Plugin   = 7,
}

/// A single searchable entry in the launcher index.
/// Serialized to the frontend via Tauri's IPC bridge.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    /// Display name shown in the results list.
    pub name: String,
    /// Pre-computed lowercase name for O(1) case-insensitive matching.
    pub name_lower: String,
    /// Absolute path on disk, or a JSON payload for workflows.
    pub path: String,
    /// Visual subtitle shown below the name (e.g. parent dir).
    pub subtitle: String,
    /// Classification used for icon rendering and launch behavior.
    pub kind: EntryKind,
    /// Static score bias (system actions get a boost).
    pub score: u32,
    #[serde(default)]
    pub search_score: i64,
}

/// A user-defined workflow (Alfred Powerpack equivalent).
/// Stored in the config file and injected into the index on boot.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Workflow {
    pub name: String,
    pub keyword: Option<String>,
    pub hotkey: Option<String>,
    pub command: String,
    pub args: Vec<String>,
}

/// Top-level configuration, persisted to %AppData%/jor/config.json.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub workflows: Vec<Workflow>,
    /// Directories to index in addition to the defaults.
    pub extra_paths: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workflows: vec![
                Workflow {
                    name: "Terminal".into(),
                    keyword: Some("terminal".into()),
                    hotkey: Some("alt+t".into()),
                    command: "wt".into(),
                    args: vec![],
                },
                Workflow {
                    name: "Open Brave Incognito".into(),
                    keyword: Some("brave-incognito".into()),
                    hotkey: None,
                    command: "brave".into(), // or "brave-browser" / full path depending on your system
                    args: vec!["--incognito".into()],
                },
            ],
            extra_paths: vec![],
        }
    }
}
