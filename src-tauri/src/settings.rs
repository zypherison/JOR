use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeConfig {
    pub bg_top: String,
    pub bg_mid: String,
    pub bg_bottom: String,
    pub accent: String,
    pub panel_border: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            bg_top: "#2a2d31".into(),
            bg_mid: "#23262b".into(),
            bg_bottom: "#1b1e22".into(),
            accent: "#d7dce2".into(),
            panel_border: "rgba(236, 236, 236, 0.2)".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub theme: ThemeConfig,
    pub enabled_plugins: Vec<String>,
    pub main_hotkey: String,
    pub clip_hotkey: String,
    pub custom_hotkeys: HashMap<String, String>,
    pub terms_accepted: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            enabled_plugins: vec!["clipboard".into(), "converter".into(), "window_manager".into()],
            main_hotkey: "Alt+Space".into(),
            clip_hotkey: "Alt+V".into(),
            custom_hotkeys: HashMap::new(),
            terms_accepted: false,
        }
    }
}

pub fn get_settings_path(app_handle: &tauri::AppHandle) -> PathBuf {
    let app_dir = app_handle.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
    if !app_dir.exists() {
        let _ = fs::create_dir_all(&app_dir);
    }
    app_dir.join("settings.json")
}

pub fn load_settings(app_handle: &tauri::AppHandle) -> Settings {
    let path = get_settings_path(app_handle);
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(settings) = serde_json::from_str(&content) {
            return settings;
        }
    }
    Settings::default()
}

pub fn save_settings(app_handle: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let path = get_settings_path(app_handle);
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}
