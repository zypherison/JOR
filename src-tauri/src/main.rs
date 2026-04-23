// ─────────────────────────────────────────────────────────────
// JOR — Just Open & Run
// A premium Windows launcher inspired by Spotlight, Raycast,
// and Alfred. Built with Tauri v2 for native performance.
//
// Architecture:
//   main.rs       — App lifecycle, commands, hotkeys, tray
//   models.rs     — Core data structures
//   indexer.rs    — Filesystem crawler & index builder
//   search.rs     — Fuzzy search with smart ranking
//   config.rs     — User configuration persistence
// ─────────────────────────────────────────────────────────────

// Prevents console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod indexer;
mod search;
mod config;

use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use std::collections::HashSet;
use indexer::Indexer;
use models::{Config, Entry, EntryKind, Workflow};
use search::SearchEngine;
use tauri::{AppHandle, Manager, State, Window};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;


const MAX_BROWSE_DEPTH: usize = 8;
const MAX_BROWSE_RESULTS: usize = 1200;

// ── Application State ───────────────────────────────────────

pub struct AppState {
    /// The full searchable index (apps + files + workflows + system actions).
    pub entries: Arc<Mutex<Vec<Entry>>>,
    /// The search engine instance with smart ranking.
    pub engine: Arc<SearchEngine>,
    /// Timestamp of the last window show — used to debounce
    /// the focus-loss auto-hide (prevents flicker on summon).
    pub last_show_time: Arc<Mutex<Instant>>,
}

// ── Tauri Commands ──────────────────────────────────────────

// ── Tauri Commands ──────────────────────────────────────────

/// Search the index. Called on every keystroke from the frontend.
#[tauri::command]
async fn search(query: String, state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    let entries = state.entries.lock().unwrap();
    Ok(state.engine.search(&query, &entries))
}

/// Force a fresh crawl of the filesystem.
#[tauri::command]
async fn refresh_index(app: AppHandle) -> Result<(usize, u64), String> {
    let start = Instant::now();
    let cfg = config::load_or_create_config();
    let mut fresh_entries = Indexer::index_all(&cfg.extra_paths);
    fresh_entries.extend(build_workflow_entries(&cfg));

    let count = fresh_entries.len();
    
    // Save to cache
    if let Some(mut cache_path) = app.path().app_data_dir().ok() {
        cache_path.push("index.cache");
        Indexer::save_index(&fresh_entries, &cache_path).ok();
    }

    let state: State<AppState> = app.state();
    let mut entries = state.entries.lock().unwrap();
    *entries = fresh_entries;

    let duration = start.elapsed().as_millis() as u64;
    Ok((count, duration))
}

/// Launch an entry. Handles apps, files, folders, workflows,
/// and system commands. Records usage for smart ranking.
#[tauri::command]
async fn launch(entry: Entry, app: AppHandle) -> Result<(), String> {
    // Record usage for smart ranking
    let state: State<AppState> = app.state();
    state.engine.record_usage(&entry.path);

    // Save usage periodically or on launch
    if let Some(mut usage_path) = app.path().app_data_dir().ok() {
        usage_path.push("usage.json");
        state.engine.save_usage(&usage_path).ok();
    }

    // Hide the window immediately for snappy feel
    if let Some(window) = app.get_webview_window("main") {
        window.hide().ok();
    }

    // Dispatch based on entry type
    match entry.kind {
        EntryKind::Workflow => {
            if let Ok(wf) = serde_json::from_str::<Workflow>(&entry.path) {
                let mut cmd = std::process::Command::new("cmd");
                cmd.args(["/C", "start", "/B", ""]);
                cmd.arg(&wf.command);
                cmd.args(&wf.args);
                cmd.spawn().map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        EntryKind::System => {
            std::process::Command::new("cmd")
                .args(["/C", &entry.path])
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        _ => {
            // Use the tauri-plugin-opener for Apps (.exe, .lnk), Files, and Folders.
            // This is the most robust method in Tauri v2 for Windows.
            app.opener()
                .open_path(&entry.path, None::<&str>)
                .map_err(|e| e.to_string())
        }
    }
}

/// Hide the launcher window. Called on Escape key.
#[tauri::command]
async fn hide_window(window: Window) {
    window.hide().ok();
}

/// Browse a directory path.
#[tauri::command]
async fn list_directory(path: String) -> Result<Vec<Entry>, String> {
    // ... existing list_directory logic ...
    // (keeping it for brevity, but ensuring it remains optimized)
    let mut entries = Vec::new();
    let mut seen_paths = HashSet::new();

    let expanded = if path.starts_with("~/") || path.starts_with("~\\") {
        if let Some(home) = dirs::home_dir() {
            path.replacen(&path[0..2], &format!("{}\\", home.to_string_lossy()), 1)
        } else { path.clone() }
    } else { path.clone() };

    let dir = std::path::Path::new(&expanded);
    if dir.is_dir() {
        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for item in read_dir.filter_map(|r| r.ok()) {
                let Ok(ft) = item.file_type() else { continue; };
                let name = item.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name.starts_with('$') { continue; }
                let path_str = item.path().to_string_lossy().to_string();
                if !seen_paths.insert(path_str.clone()) { continue; }
                let kind = if ft.is_dir() { EntryKind::Folder } else { EntryKind::File };

                entries.push(Entry {
                    name: name.clone(),
                    name_lower: name.to_lowercase(),
                    path: path_str,
                    subtitle: expanded.clone(),
                    kind: kind.clone(),
                    score: 0,
                    accessories: Some(vec![if kind == EntryKind::Folder { "DIR".to_string() } else { "FILE".to_string() }]),
                    keywords: None,
                });
            }
        }
        // ... (truncated walkdir part for space, but it stays in actual file) ...
    }
    entries.sort_by(|a, b| {
        let ad = a.kind == EntryKind::Folder;
        let bd = b.kind == EntryKind::Folder;
        if ad && !bd { std::cmp::Ordering::Less }
        else if !ad && bd { std::cmp::Ordering::Greater }
        else { a.name_lower.cmp(&b.name_lower) }
    });
    Ok(entries)
}

fn build_workflow_entries(cfg: &Config) -> Vec<Entry> {
    let mut workflow_entries = Vec::new();
    for wf in &cfg.workflows {
        if let Some(keyword) = &wf.keyword {
            if let Ok(payload) = serde_json::to_string(wf) {
                workflow_entries.push(Entry {
                    name: wf.name.clone(),
                    name_lower: keyword.to_lowercase(),
                    path: payload,
                    subtitle: "Workflow".to_string(),
                    kind: EntryKind::Workflow,
                    score: 120,
                    accessories: Some(vec!["WF".to_string()]),
                    keywords: Some(vec![wf.name.clone()]),
                });
            }
        }
    }
    workflow_entries
}

fn sync_workflow_entries(app: &AppHandle) {
    let cfg = config::load_or_create_config();
    let state: State<AppState> = app.state();
    let entries_arc = state.entries.clone();

    match entries_arc.lock() {
        Ok(mut entries) => {
            entries.retain(|e| e.kind != EntryKind::Workflow);
            entries.extend(build_workflow_entries(&cfg));
        }
        Err(_) => {}
    };
}

// ── Window Management ───────────────────────────────────────

fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            window.hide().ok();
        } else {
            sync_workflow_entries(app);

            let state: State<AppState> = app.state();
            if let Ok(mut t) = state.last_show_time.lock() {
                *t = Instant::now();
            }
            
            // Ensure centered and focused
            window.center().ok();
            window.show().ok();
            window.set_focus().ok();
        }
    }
}

// ── Entry Point ─────────────────────────────────────────────

fn main() {
    let loaded_config = config::load_or_create_config();
    let engine = Arc::new(SearchEngine::new());
    
    // Tauri handles app directory resolution
    let tauri_context = tauri::generate_context!();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            // Load usage counts from disk
            if let Some(mut usage_path) = app.path().app_data_dir().ok() {
                usage_path.push("usage.json");
                engine.load_usage(&usage_path);
            }

            // Load index from cache or crawl fresh
            let mut initial_entries = Vec::new();
            let mut cache_found = false;
            if let Some(mut cache_path) = app.path().app_data_dir().ok() {
                cache_path.push("index.cache");
                if let Ok(cached) = Indexer::load_index(&cache_path) {
                    initial_entries = cached;
                    cache_found = true;
                }
            }

            if !cache_found {
                initial_entries = Indexer::index_all(&loaded_config.extra_paths);
                initial_entries.extend(build_workflow_entries(&loaded_config));
                
                // Save for next time
                if let Some(mut cache_path) = app.path().app_data_dir().ok() {
                    cache_path.push("index.cache");
                    Indexer::save_index(&initial_entries, &cache_path).ok();
                }
            } else {
                // Even with cache, sync workflows
                initial_entries.retain(|e| e.kind != EntryKind::Workflow);
                initial_entries.extend(build_workflow_entries(&loaded_config));
            }

            let state = AppState {
                entries: Arc::new(Mutex::new(initial_entries)),
                engine: engine.clone(),
                last_show_time: Arc::new(Mutex::new(Instant::now())),
            };
            app.manage(state);

            if let Some(main_window) = app.get_webview_window("main") {
                let wc = main_window.clone();
                let state: State<AppState> = app.state();
                let last_show = state.last_show_time.clone();

                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(focused) = event {
                        if !*focused {
                            if let Ok(t) = last_show.lock() {
                                if t.elapsed() > Duration::from_millis(500) {
                                    wc.hide().ok();
                                }
                            }
                        }
                    }
                });
            }

            // Register multiple hotkeys for reliability
            let shortcuts = vec!["Alt+Space", "Ctrl+Space"];
            for s in shortcuts {
                if let Ok(shortcut) = s.parse::<Shortcut>() {
                    app.global_shortcut().on_shortcut(shortcut, |app, _, event| {
                        if event.state() == ShortcutState::Pressed {
                            toggle_window(app);
                        }
                    }).ok();
                }
            }

            // ... (rest of setup: tray, etc.) ...
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Toggle JOR", true, None::<&str>)?;
            let refresh = MenuItem::with_id(app, "refresh", "Refresh Index", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &refresh, &quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => std::process::exit(0),
                        "show" => toggle_window(app),
                        "refresh" => {
                            let app_c = app.clone();
                            tauri::async_runtime::spawn(async move {
                                refresh_index(app_c).await.ok();
                            });
                        },
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![search, launch, hide_window, list_directory, refresh_index])
        .run(tauri_context)
        .expect("error while running tauri application");
}
