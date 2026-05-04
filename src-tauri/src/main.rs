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
mod plugins;
mod settings;

use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::collections::HashSet;
use crate::plugins::Plugin;
use indexer::Indexer;
use models::{Config, Entry, EntryKind, Workflow};
use search::SearchEngine;
use settings::Settings;
use tauri::{AppHandle, Manager, Wry, State, Window, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;
use walkdir::WalkDir;

const MAX_BROWSE_DEPTH: usize = 8;
const MAX_BROWSE_RESULTS: usize = 1200;

// ── Application State ───────────────────────────────────────

pub struct AppState {
    pub entries: Arc<Mutex<Vec<Entry>>>,
    pub engine: Arc<SearchEngine>,
    pub last_show_time: Arc<Mutex<Instant>>,
    pub plugins: Vec<Box<dyn Plugin + Send + Sync>>,
    pub settings: Arc<Mutex<Settings>>,
}

// ── Tauri Commands ──────────────────────────────────────────

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
async fn update_settings(new_settings: Settings, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Save to disk
    settings::save_settings(&app, &new_settings)?;
    
    // Update state
    {
        let mut s = state.settings.lock().unwrap();
        *s = new_settings.clone();
    }
    
    // Apply changes (hotkeys, etc.)
    apply_settings(&app, &new_settings).await?;
    
    Ok(())
}

async fn apply_settings(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    // 1. Update hotkeys
    let shortcut_plugin = app.global_shortcut();
    shortcut_plugin.unregister_all().map_err(|e| e.to_string())?;

    // Register Main JOR
    if let Ok(s) = settings.main_hotkey.parse::<Shortcut>() {
        let _ = shortcut_plugin.on_shortcut(s, |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                toggle_window(app);
            }
        });
    }

    // Register Clip JOR
    if let Ok(s) = settings.clip_hotkey.parse::<Shortcut>() {
        let _ = shortcut_plugin.on_shortcut(s, |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                toggle_clipboard_window(app);
            }
        });
    }

    // Register Custom Hotkeys
    for (hk, path) in &settings.custom_hotkeys {
        if let Ok(s) = hk.parse::<Shortcut>() {
            let p = path.clone();
            let _ = shortcut_plugin.on_shortcut(s, move |app, _, event| {
                if event.state() == ShortcutState::Pressed {
                    let p_clone = p.clone();
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let mut matched_entry = None;
                        if let Some(state) = app_clone.try_state::<AppState>() {
                            if let Ok(entries) = state.entries.lock() {
                                if let Some(e) = entries.iter().find(|e| e.path == p_clone) {
                                    matched_entry = Some(e.clone());
                                }
                            }
                        }
                        
                        if let Some(entry) = matched_entry {
                            let _ = launch(entry, app_clone).await;
                        } else {
                            let _ = app_clone.opener().open_path(&p_clone, None::<&str>);
                        }
                    });
                }
            });
        }
    }

    // 2. Notify windows to update theme
    app.emit("theme-changed", &settings.theme).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn clear_clipboard_history(state: State<'_, AppState>) -> Result<(), String> {
    for plugin in &state.plugins {
        if plugin.id() == "clipboard" {
            // We need to execute a special action to clear
            return plugin.execute("clear_all").await;
        }
    }
    Err("Clipboard plugin not found".into())
}

use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON, GetIconInfo, ICONINFO};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V, VK_MENU, VK_SHIFT
};
use windows::Win32::Graphics::Gdi::{
    GetDC, ReleaseDC, GetDIBits, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
    CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, DeleteDC, DeleteObject
};
use windows::Win32::Foundation::HWND;
use std::os::windows::ffi::OsStrExt;
use base64::{Engine as _, engine::general_purpose};

#[tauri::command]
async fn get_app_icon(path: String) -> Result<String, String> {
    let mut path_wide: Vec<u16> = std::path::Path::new(&path).as_os_str().encode_wide().collect();
    path_wide.push(0);

    let mut large_icon = [HICON(0)];
    unsafe {
        ExtractIconExW(
            windows::core::PCWSTR(path_wide.as_ptr()),
            0,
            Some(large_icon.as_mut_ptr()),
            None,
            1
        );

        if large_icon[0].is_invalid() {
            return Err("Icon not found".into());
        }

        let hicon = large_icon[0];
        let mut icon_info = ICONINFO::default();
        GetIconInfo(hicon, &mut icon_info).map_err(|e| e.to_string())?;

        let hdc_screen = GetDC(HWND(0));
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm_mem = CreateCompatibleBitmap(hdc_screen, 32, 32);
        let hold_bm = SelectObject(hdc_mem, hbm_mem);

        // Fill background with transparency/specific color if needed, 
        // but DI_NORMAL handles most transparency cases into the bitmap.
        windows::Win32::UI::WindowsAndMessaging::DrawIconEx(
            hdc_mem, 0, 0, hicon, 32, 32, 0, None,
            windows::Win32::UI::WindowsAndMessaging::DI_NORMAL
        ).map_err(|e| e.to_string())?;

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 32,
                biHeight: -32, // Top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut buffer = vec![0u8; 32 * 32 * 4];
        GetDIBits(hdc_mem, hbm_mem, 0, 32, Some(buffer.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);

        // Clean up immediately
        SelectObject(hdc_mem, hold_bm);
        DeleteObject(hbm_mem);
        DeleteDC(hdc_mem);
        ReleaseDC(HWND(0), hdc_screen);
        let _ = DeleteObject(icon_info.hbmColor);
        let _ = DeleteObject(icon_info.hbmMask);
        let _ = DestroyIcon(hicon);

        // BMP file header (14 bytes) + DIB header (40 bytes)
        let mut bmp_file = vec![
            0x42, 0x4D,             // Signature "BM"
            0x36, 0x10, 0x00, 0x00, // File size (4150 bytes)
            0x00, 0x00, 0x00, 0x00, // Reserved
            0x36, 0x00, 0x00, 0x00, // Data offset (54 bytes)
            0x28, 0x00, 0x00, 0x00, // DIB Header size (40 bytes)
            0x20, 0x00, 0x00, 0x00, // Width (32)
            0xE0, 0xFF, 0xFF, 0xFF, // Height (-32, top-down)
            0x01, 0x00,             // Planes (1)
            0x20, 0x00,             // Bits per pixel (32)
            0x00, 0x00, 0x00, 0x00, // Compression (None)
            0x00, 0x10, 0x00, 0x00, // Image size (4096 bytes)
            0x00, 0x00, 0x00, 0x00, // XpixelsPerM
            0x00, 0x00, 0x00, 0x00, // YpixelsPerM
            0x00, 0x00, 0x00, 0x00, // Colors used
            0x00, 0x00, 0x00, 0x00, // Important colors
        ];
        
        // GDI buffer is already BGRA, which BMP expects.
        bmp_file.extend(buffer);
        let b64 = general_purpose::STANDARD.encode(bmp_file);
        Ok(format!("data:image/bmp;base64,{}", b64))
    }
}

#[tauri::command]
async fn accept_terms(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    let mut s = state.settings.lock().unwrap();
    s.terms_accepted = true;
    settings::save_settings(&app, &s)?;
    
    if let Some(w) = app.get_webview_window("tos") {
        w.hide().ok();
    }
    if let Some(w) = app.get_webview_window("settings") {
        w.show().ok();
    }
    Ok(())
}

#[tauri::command]
async fn search(query: String, mode: String, state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    if mode == "clipboard" {
        // STRICT: Only clipboard results
        let mut results = Vec::new();
        for plugin in &state.plugins {
            if plugin.id() == "clipboard" {
                results.extend(plugin.search(&query, &mode).await);
            }
        }
        return Ok(results);
    }

    // Standard mode: Search engine (Apps/Files) + Non-clipboard plugins
    let mut results = {
        let entries = state.entries.lock().unwrap();
        state.engine.search(&query, &entries)
    };

    // Query relevant plugins (excluding clipboard for standard mode)
    for plugin in &state.plugins {
        if plugin.id() != "clipboard" {
            let plugin_results = plugin.search(&query, &mode).await;
            results.extend(plugin_results);
        }
    }

    // Sort again to ensure plugin results are prioritized by score
    results.sort_by(|a, b| b.search_score.cmp(&a.search_score));
    
    // Final limit to 50 results for performance
    results.truncate(50);

    Ok(results)
}

#[tauri::command]
async fn launch(entry: Entry, app: AppHandle) -> Result<(), String> {
    // Record usage for smart ranking
    let state: State<AppState> = app.state();
    state.engine.record_usage(&entry.path);

    // Hide the window immediately
    if let Some(window) = app.get_webview_window("main") {
        window.hide().ok();
    }
    if let Some(window) = app.get_webview_window("clipboard") {
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
            std::process::Command::new("cmd")
                .args(["/C", &entry.path])
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        EntryKind::Plugin => {
            if let Some((plugin_id, action_id)) = entry.path.split_once(':') {
                for plugin in &state.plugins {
                    if plugin.id() == plugin_id {
                        plugin.execute(action_id).await.map_err(|e| e.to_string())?;
                        
                        // If it was the clipboard plugin, simulate paste
                        if plugin_id == "clipboard" {
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(300));
                                unsafe {
                                    let mut inputs = [INPUT::default(); 6];
                                    inputs[0].r#type = INPUT_KEYBOARD;
                                    inputs[0].Anonymous.ki = KEYBDINPUT { wVk: VK_MENU, dwFlags: KEYEVENTF_KEYUP, ..Default::default() };
                                    inputs[1].r#type = INPUT_KEYBOARD;
                                    inputs[1].Anonymous.ki = KEYBDINPUT { wVk: VK_SHIFT, dwFlags: KEYEVENTF_KEYUP, ..Default::default() };
                                    inputs[2].r#type = INPUT_KEYBOARD;
                                    inputs[2].Anonymous.ki = KEYBDINPUT { wVk: VK_CONTROL, ..Default::default() };
                                    inputs[3].r#type = INPUT_KEYBOARD;
                                    inputs[3].Anonymous.ki = KEYBDINPUT { wVk: VK_V, ..Default::default() };
                                    inputs[4].r#type = INPUT_KEYBOARD;
                                    inputs[4].Anonymous.ki = KEYBDINPUT { wVk: VK_V, dwFlags: KEYEVENTF_KEYUP, ..Default::default() };
                                    inputs[5].r#type = INPUT_KEYBOARD;
                                    inputs[5].Anonymous.ki = KEYBDINPUT { wVk: VK_CONTROL, dwFlags: KEYEVENTF_KEYUP, ..Default::default() };
                                    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                                }
                            });
                        }
                        
                        return Ok(());
                    }
                }
            }
            Err("Plugin not found".into())
        }
        _ => {
            app.opener()
                .open_path(&entry.path, None::<&str>)
                .map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
async fn hide_window(window: Window) {
    window.hide().ok();
}

#[tauri::command]
async fn list_directory(path: String) -> Result<Vec<Entry>, String> {
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
                    kind,
                    score: 0,
                    search_score: 0,
                });
            }
        }

        for item in WalkDir::new(dir)
            .min_depth(2)
            .max_depth(MAX_BROWSE_DEPTH)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entries.len() >= MAX_BROWSE_RESULTS { break; }
            let path = item.path();
            let Some(name_os) = path.file_name() else { continue; };
            let name = name_os.to_string_lossy().to_string();
            if name.starts_with('.') || name.starts_with('$') { continue; }
            let path_str = path.to_string_lossy().to_string();
            if !seen_paths.insert(path_str.clone()) { continue; }
            let kind = if path.is_dir() { EntryKind::Folder } else { EntryKind::File };
            let subtitle = path.strip_prefix(dir).ok().and_then(|p| p.parent()).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| expanded.clone());

            entries.push(Entry {
                name: name.clone(),
                name_lower: name.to_lowercase(),
                path: path_str,
                subtitle,
                kind,
                score: 0,
                search_score: 0,
            });
        }
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
                    search_score: 0,
                });
            }
        }
    }
    workflow_entries
}

fn sync_workflow_entries(app: &AppHandle) {
    let cfg = config::load_or_create_config();
    let state = app.state::<AppState>();
    if let Ok(mut entries) = state.entries.lock() {
        entries.retain(|e| e.kind != EntryKind::Workflow);
        entries.extend(build_workflow_entries(&cfg));
    };
}

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
            window.emit("switch-mode", "standard").ok();
            window.show().ok();
            window.set_focus().ok();
        }
    }
}

fn toggle_clipboard_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("clipboard") {
        if window.is_visible().unwrap_or(false) {
            window.hide().ok();
        } else {
            window.show().ok();
            window.set_focus().ok();
        }
    }
}

#[tauri::command]
async fn get_executable_list(state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    let entries = state.entries.lock().map_err(|e| e.to_string())?;
    let mut apps: Vec<Entry> = entries.iter()
        .filter(|e| e.kind == EntryKind::App || e.kind == EntryKind::System)
        .cloned()
        .collect();
        
    // Add common system settings
    let settings = [
        ("Wi-Fi Settings", "ms-settings:network-wifi", "System Settings"),
        ("Bluetooth Settings", "ms-settings:bluetooth", "System Settings"),
        ("Display Settings", "ms-settings:display", "System Settings"),
        ("Sound Settings", "ms-settings:mmsys", "System Settings"),
        ("Windows Update", "ms-settings:windowsupdate", "System Settings"),
        ("Battery Settings", "ms-settings:batterysaver", "System Settings"),
        ("Personalization", "ms-settings:personalization", "System Settings"),
        ("Add or Remove Programs", "ms-settings:appsfeatures", "System Settings"),
        ("Date & Time", "ms-settings:dateandtime", "System Settings"),
        ("Privacy Settings", "ms-settings:privacy", "System Settings"),
        ("Search Settings", "ms-settings:search", "System Settings"),
        ("Task Manager", "taskmgr", "System Utility"),
        ("Control Panel", "control", "System Utility"),
        ("Registry Editor", "regedit", "System Utility"),
        ("Services", "services.msc", "System Utility"),
        ("Command Prompt", "cmd", "System Utility"),
        ("PowerShell", "powershell", "System Utility"),
    ];
    
    for (name, path, sub) in settings {
        apps.push(Entry {
            name: name.to_string(),
            name_lower: name.to_lowercase(),
            path: path.to_string(),
            subtitle: sub.to_string(),
            kind: EntryKind::System,
            score: 0,
            search_score: 0,
        });
    }

    apps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(apps)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            let loaded_config = config::load_or_create_config();
            let mut initial_entries = Indexer::index_all(app.handle(), &loaded_config.extra_paths);
            initial_entries.extend(build_workflow_entries(&loaded_config));

            let user_settings = settings::load_settings(app.handle());
            let engine = Arc::new(SearchEngine::new());

            let state = AppState {
                entries: Arc::new(Mutex::new(initial_entries)),
                engine: engine.clone(),
                last_show_time: Arc::new(Mutex::new(Instant::now())),
                plugins: vec![
                    Box::new(plugins::clipboard::ClipboardPlugin::new()),
                    Box::new(plugins::converter::ConverterPlugin),
                    Box::new(plugins::window_manager::WindowManagerPlugin),
                ],
                settings: Arc::new(Mutex::new(user_settings.clone())),
            };

            app.manage(state);
            
            // Background refresh index to keep it fresh
            indexer::Indexer::refresh_index(app.handle(), loaded_config.extra_paths.clone());

            let app_handle = app.handle();
            let app_state: State<AppState> = app_handle.state();

            // ── Terms of Service Check ──────────────────────
            if !user_settings.terms_accepted {
                if let Some(tos_window) = app.get_webview_window("tos") {
                    tos_window.show().ok();
                }
            } else {
                if let Some(settings_window) = app.get_webview_window("settings") {
                    settings_window.show().ok();
                }
            }

            for plugin in &app_state.plugins {
                plugin.init(app_handle);
            }

            let ah = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = apply_settings(&ah, &user_settings).await;
            });

            if let Some(main_window) = app.get_webview_window("main") {
                let wc = main_window.clone();
                let last_show = app_state.last_show_time.clone();
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(focused) = event {
                        if !*focused {
                            if let Ok(t) = last_show.lock() {
                                if t.elapsed().as_millis() > 100 { wc.hide().ok(); }
                            }
                        }
                    }
                });
            }

            if let Some(clip_window) = app.get_webview_window("clipboard") {
                let wc = clip_window.clone();
                clip_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(focused) = event {
                        if !*focused { wc.hide().ok(); }
                    }
                });
            }

            // Tray
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
        .invoke_handler(tauri::generate_handler![
            search,
            launch,
            hide_window,
            list_directory,
            get_settings,
            update_settings,
            clear_clipboard_history,
            accept_terms,
            get_app_icon,
            get_executable_list
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
