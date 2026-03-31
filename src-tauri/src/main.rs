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
use tauri::{AppHandle, Manager, Wry, State, Window};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;
use walkdir::WalkDir;

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

/// Search the index. Called on every keystroke from the frontend.
#[tauri::command]
async fn search(query: String, state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    let entries = state.entries.lock().unwrap();
    Ok(state.engine.search(&query, &entries))
}

/// Launch an entry. Handles apps, files, folders, workflows,
/// and system commands. Records usage for smart ranking.
#[tauri::command]
async fn launch(entry: Entry, app: AppHandle) -> Result<(), String> {
    // Record usage for smart ranking
    let state: State<AppState> = app.state();
    state.engine.record_usage(&entry.path);

    // Hide the window immediately for snappy feel
    if let Some(window) = app.get_webview_window("main") {
        window.hide().ok();
    }

    // Dispatch based on entry type
    match entry.kind {
        EntryKind::Workflow => {
            if let Ok(wf) = serde_json::from_str::<Workflow>(&entry.path) {
                std::process::Command::new(&wf.command)
                    .args(&wf.args)
                    .spawn()
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        EntryKind::System => {
            // System commands run via cmd /c for shell interpretation
            std::process::Command::new("cmd")
                .args(["/C", &entry.path])
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        _ => {
            // Apps, files, folders — open with system default handler
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

/// Browse a directory path. Called when the user types a path
/// (e.g. "C:\\" or "~/"). Returns contents sorted dirs-first.
#[tauri::command]
async fn list_directory(path: String) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    let mut seen_paths = HashSet::new();

    // Resolve ~ to home directory
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
                // Skip hidden items
                if name.starts_with('.') || name.starts_with('$') { continue; }
                let path_str = item.path().to_string_lossy().to_string();
                if !seen_paths.insert(path_str.clone()) { continue; }
                let kind = if ft.is_dir() { EntryKind::Folder } else { EntryKind::File };

                entries.push(Entry {
                    name: name.clone(),
                    name_lower: name.to_lowercase(),
                    path: path_str,
                    subtitle: expanded.clone(),
                    kind,
                    score: 0,
                });
            }
        }

        // Include nested descendants to make deep folder content discoverable.
        // Direct children are still listed first after sorting.
        for item in WalkDir::new(dir)
            .min_depth(2)
            .max_depth(MAX_BROWSE_DEPTH)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entries.len() >= MAX_BROWSE_RESULTS {
                break;
            }

            let path = item.path();
            let Some(name_os) = path.file_name() else { continue; };
            let name = name_os.to_string_lossy().to_string();
            if name.starts_with('.') || name.starts_with('$') { continue; }

            let path_str = path.to_string_lossy().to_string();
            if !seen_paths.insert(path_str.clone()) { continue; }

            let kind = if path.is_dir() { EntryKind::Folder } else { EntryKind::File };
            let subtitle = path
                .strip_prefix(dir)
                .ok()
                .and_then(|p| p.parent())
                .map(|p| {
                    let s = p.to_string_lossy();
                    if s.is_empty() { expanded.clone() } else { s.to_string() }
                })
                .unwrap_or_else(|| expanded.clone());

            entries.push(Entry {
                name: name.clone(),
                name_lower: name.to_lowercase(),
                path: path_str,
                subtitle,
                kind,
                score: 0,
            });
        }
    }

    // Sort: folders first, then alphabetical
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
                });
            }
        }
    }
    workflow_entries
}

fn sync_workflow_entries(app: &AppHandle) {
    let cfg = config::load_or_create_config();
    let entries_arc = {
        let state: State<AppState> = app.state();
        state.entries.clone()
    };

    match entries_arc.lock() {
        Ok(mut entries) => {
            entries.retain(|e| e.kind != EntryKind::Workflow);
            entries.extend(build_workflow_entries(&cfg));
        }
        Err(_) => {}
    };
}

// ── Window Management ───────────────────────────────────────

/// Toggle the launcher window visibility.
/// On show: records timestamp for debouncing.
fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            window.hide().ok();
        } else {
            // Pull latest workflow config before each show so deletions/edits apply immediately.
            sync_workflow_entries(app);

            let state: State<AppState> = app.state();
            if let Ok(mut t) = state.last_show_time.lock() {
                *t = Instant::now();
            }
            window.show().ok();
            window.set_focus().ok();
        }
    }
}

// ── Entry Point ─────────────────────────────────────────────

fn main() {
    // Load user configuration
    let loaded_config = config::load_or_create_config();

    // Build fresh index on every launch to stay current
    let mut initial_entries = Indexer::index_all(&loaded_config.extra_paths);

    // Inject user-defined workflows as searchable entries.
    initial_entries.extend(build_workflow_entries(&loaded_config));

    let engine = Arc::new(SearchEngine::new());

    let state = AppState {
        entries: Arc::new(Mutex::new(initial_entries)),
        engine: engine.clone(),
        last_show_time: Arc::new(Mutex::new(Instant::now())),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            app.manage(state);

            // ── Focus-loss auto-hide (Alt-V pattern) ────────
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

            // ── Global hotkey: Alt+Space ────────────────────
            if let Ok(shortcut) = "Alt+Space".parse::<Shortcut>() {
                app.global_shortcut().on_shortcut(shortcut, |app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_window(app);
                    }
                }).ok();
            }

            // ── Dynamic workflow hotkeys ────────────────────
            for wf in &loaded_config.workflows {
                if let Some(hk) = &wf.hotkey {
                    if let Ok(shortcut) = hk.parse::<Shortcut>() {
                        let cmd = wf.command.clone();
                        let args = wf.args.clone();
                        app.global_shortcut().on_shortcut(shortcut, move |_, _, event| {
                            if event.state() == ShortcutState::Pressed {
                                std::process::Command::new(&cmd).args(&args).spawn().ok();
                            }
                        }).ok();
                    }
                }
            }

            // ── System tray ─────────────────────────────────
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Toggle JOR", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let _tray = TrayIconBuilder::<Wry>::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => std::process::exit(0),
                        "show" => toggle_window(app),
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![search, launch, hide_window, list_directory])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
