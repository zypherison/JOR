#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;

use tauri::{AppHandle, Manager, State, Window};
use crate::models::{Entry, EntryKind};
use std::sync::Mutex;

struct AppState {
    index: Mutex<Vec<Entry>>,
}

#[tauri::command]
async fn search(query: String, state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    let index = state.index.lock().unwrap();
    if query.is_empty() {
        return Ok(index.iter().take(20).cloned().collect());
    }
    
    let query_lower = query.toLowerCase();
    let mut results: Vec<Entry> = index.iter()
        .filter(|e| e.name_lower.contains(&query_lower))
        .take(20)
        .cloned()
        .collect();
    
    Ok(results)
}

#[tauri::command]
async fn hide_window(window: Window) {
    window.hide().unwrap();
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            index: Mutex::new(vec![]),
        })
        .invoke_handler(tauri::generate_handler![search, hide_window])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            
            // Global shortcut logic would go here in a full implementation
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
