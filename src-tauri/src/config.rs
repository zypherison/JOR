// ─────────────────────────────────────────────────────────────
// JOR — Configuration Manager
// Loads or creates %AppData%/jor/config.json with user
// workflows, hotkey bindings, and extra indexed paths.
// ─────────────────────────────────────────────────────────────

use std::fs;
use std::path::PathBuf;
use crate::models::Config;

/// Returns the path to the JOR config directory.
pub fn config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("C:\\"));
    base.join("jor")
}

/// Load existing config or create a default one on first run.
pub fn load_or_create_config() -> Config {
    let dir = config_dir();

    if !dir.exists() {
        fs::create_dir_all(&dir).ok();
    }

    let config_path = dir.join("config.json");
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }
    }

    // First run — write defaults
    let default_config = Config::default();
    if let Ok(json) = serde_json::to_string_pretty(&default_config) {
        fs::write(&config_path, json).ok();
    }

    default_config
}
