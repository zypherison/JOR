<div align="center">
  <h1>🚀 JOR: Just Open & Run</h1>
  <p><strong>The blazingly fast, Gen-Z styled app launcher for Windows.</strong></p>
  <img src="https://img.shields.io/badge/built%20with-Tauri%20v2-24c8db?style=for-the-badge&logo=tauri" alt="Built with Tauri" />
  <img src="https://img.shields.io/badge/language-Rust%20%2B%20TypeScript-000000?style=for-the-badge&logo=rust" alt="Rust & TS" />
</div>

<br />

Say goodbye to sluggish Windows Search and visually outdated launchers. **JOR** (Just Open & Run) is a highly optimized, minimalist Spotlight-style app launcher crafted strictly targeting performance and modern aesthetics.

Made for the generation that hates loading screens.

## ✨ Features
- **Ultra-Fast Backend**: Indexed parsing using native Rust components via `bincode` serialization and `<10ms` fuzzy matching times. 
- **Glassmorphic UI**: Powered by Vanilla HTML and CSS inside an optimized webview. Modern and buttery smooth.
- **Workflow / Hotkey Engine**: Mimicking Raycast and Alfred Powerpacks for *free*. Add specific shell commands, launch args, or workflows bound directly to keywords or global hotkeys seamlessly via the background Daemon.
- **Custom Configs**: Everything is parsed automatically without needing UI bloat via `%AppData%\Roaming\jor\config.json`.
- **System Commands integration**: Lock, Sleep, Restart directly from your search!
- **Math Solver & Web Parsing**: Resolves exact equations instantly right into your clipboard! (e.g., `45 * 2`)
- **Background Daemon Engine**: Target hitting `Alt+Space` seamlessly. Stays loaded in memory for near zero CPU.

## ⚙️ How It Works
When you trigger JOR, it avoids the heavy latency of standard APIs by matching against a highly compressed, pre-compiled file manifest index. Launch operations natively interface with Windows internals (`ShellExecute`) by invoking explicit native handles.

## 🛠️ Tech Stack & Architecture
- **Tauri v2**: Powers windowing transparency, webview execution, and native bridging.
- **Rust Core**: Drives data iteration, in-memory filtering out of `<walkdir>`, and direct OS calls.
- **Vanilla TS & HTML/CSS**: No heavy React or framework bundling for maximum GUI responsiveness.

## 🚀 Building / Development
JOR uses Node package manager mixed with Cargo for dependencies. You only need to run standard Tauri commands.

```bash
# 1. Install dependencies
npm install

# 2. Run in dev mode (Hot-module reloading)
npm run tauri dev

# 3. Build optimized MSI/EXE installer out of the box
npm run tauri build
```
