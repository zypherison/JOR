<div align="center">
  <h1>🚀 JOR: Just Open & Run</h1>
  <p><strong>The blazingly fast, Gen-Z styled app launcher for Windows &amp; Linux.</strong></p>
  <img src="https://img.shields.io/badge/built%20with-Tauri%20v2-8b5cf6?style=for-the-badge&logo=tauri" alt="Built with Tauri" />
  <img src="https://img.shields.io/badge/language-Rust%20%2B%20Vanilla%20JS-000000?style=for-the-badge&logo=rust" alt="Rust & JS" />
</div>

<br />

Say goodbye to sluggish OS search and visually outdated launchers. **JOR** (Just Open & Run) is a highly optimized, minimalist Spotlight-style app launcher crafted strictly for performance and modern aesthetics.

Made for the generation that hates loading screens.

## 🖥️ Platforms

| Platform | Status | Notes |
| --- | --- | --- |
| **Windows 10/11** | ✅ Full | MSI + NSIS installers; native app icons (GDI), `SendInput` paste simulation, `ms-settings:` pages, tray + global hotkeys. |
| **Linux** | ✅ Full | AppImage + deb + rpm; `.desktop` app discovery (system + flatpak), systemctl power actions, `notify-send` timers, GNOME system utilities; falls back gracefully on non-GNOME desktops. |
| **macOS** | 🧪 Portable code | The codebase is platform-clean, but no macOS CI job or signed builds are published yet — add a `build-macos` job to the workflow to enable it. |

## ✨ Features

- **Ultra-Fast Backend**: Indexed parsing with native Rust, a cached manifest for instant startup, and fuzzy matching in single-digit milliseconds.
- **Glassmorphic UI**: Pure vanilla HTML/CSS/JS inside an optimized WebView2 window — no framework overhead. One fixed, optimized theme: no runtime theming means no extra IPC or DOM work at startup.
- **Start Menu & File Explorer Replacement**: An empty search shows quick-access folders (Home, Desktop, Documents, Downloads) plus your most-used apps. Type any path (`C:\`, `~/`, `/`) to browse folders entirely inside JOR with Tab / Enter / Backspace.
- **Built for Slow Hardware**: Static backgrounds, light blur, debounced search, and full `prefers-reduced-motion` support — no GPU-heavy animation or framework tax.
- **Plugin System**: Modular search & action plugins — enable/disable any of them from the dashboard's Features tab. Queried in parallel on every keystroke:
  - 📋 **Clipboard Manager** — persistent history (SQLite), instant search, one-click paste (auto-pastes into the last focused app).
  - 🔄 **Natural Converter** — length/weight/temperature conversions plus **live currency rates** via the Frankfurter API (ECB data), cached for 6h ("10kg to lbs", "10 usd to eur").
  - 🖥️ **System Monitor** — CPU/RAM at a glance ("cpu", "ram").
  - 🔑 **Password Generator** — secure random passwords, click to copy.
  - ⏱️ **Focus Timer** — quick countdown timers with a native notification.
  - 🎨 **Color Picker**, 🌐 **IP Checker** (real local + public IP), ⛅ **Live Weather** via Open-Meteo ("weather london"), 💬 **Text Snippets** (`!sig` → expanded text).
- **Workflow / Hotkey Engine**: Bind shell commands, launch args, or workflows to keywords or global hotkeys.
- **Settings Dashboard**: Manage hotkeys, custom app shortcuts, and per-plugin enable/disable from a dedicated window. Feature availability (e.g. the clipboard hotkey, custom app hotkeys) is driven entirely by the enabled-plugins list.
- **System Commands**: Sleep / Shut Down / Restart straight from the search bar (`rundll32`/`shutdown` on Windows, `systemctl` on Linux).
- **Math Solver & Web Parsing**: Resolves equations instantly and copies the result ("45 * 2"), plus "g <query>" Google search.
- **Background Daemon**: Stays resident in the tray, summoned with `Alt+Space`.

## 🛠️ Tech Stack & Architecture

- **Tauri v2** — windowing, transparency, native bridging, global shortcuts, tray.
- **Rust Core** — filesystem indexing, fuzzy search (`fuzzy-matcher`), clipboard DB (`rusqlite`), Windows app-icon extraction (Win32 GDI, gated to Windows-only deps), and plugin execution.
- **Vanilla JS / HTML / CSS** — no React, no bundler; pages are served straight from `src/`.

```
src-tauri/src/
  main.rs        App lifecycle, commands, hotkeys, tray
  models.rs      Core data structures
  indexer.rs     Filesystem crawler + cached index builder
  search.rs      Fuzzy search with smart ranking
  config.rs      Workflow config persistence
  settings.rs    Theme/hotkey/license settings persistence
  plugins/       Modular search/action plugins
src/
  index.html     Launcher window
  clipboard.html Clipboard history window
  settings.html  Settings dashboard window
  tos.html       First-run terms window
  js/            Shared frontend modules
```

## 📁 Configuration & Data

| What | Where |
| --- | --- |
| Workflows (`config.json`) | `%AppData%\jor\config.json` |
| Settings (`settings.json`) | Tauri app config dir (`%AppData%\com.jor.launcher\settings.json`) |
| Clipboard history (`clipboard.db`) | Tauri app data dir |
| Index cache (`index.bin`) | Tauri app cache dir |

## 🚀 Building / Development

JOR uses npm for the CLI and Cargo for Rust dependencies — standard Tauri commands only.

```bash
# 1. Install dependencies
npm install

# 2. Run in dev mode
npm run tauri dev

# 3. Build optimized MSI/EXE installers
npm run tauri build
```

> Release binaries are produced automatically by the `auto-release` GitHub Action on every push to **any** branch — each push publishes a brand-new prerelease tagged `<branch>.<iteration>` (e.g. `optimized-overhaul.1`, `optimized-overhaul.2`) carrying **Windows** (MSI + NSIS) and **Linux** (AppImage + deb + rpm) installers. Existing releases, including the stable `v1.0.0` and `v2.0.4`, are never modified or overwritten. The `release` workflow is manual-only and drives the stable `v1.0.0` winget pipeline.
