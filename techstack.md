# JOR Technical Stack & Architecture

## 1. High-Level Architecture
JOR is built using a **Windows-First Hybrid Micro-Kernel** architecture. A robust Rust-based kernel manages Windows system resources, indexing, and window lifecycle, while a high-performance web-view handles the "Cyber-Brutalist" UI rendering.

### 1.1 The JOR Kernel (Rust / Tauri v2)
- **Window Management**: Uses Tauri v2 for native Windows orchestration. Specialized in Windows 10/11 Desktop APIs for window tiling and bounds control.
- **System Mapper**: A multi-threaded Windows crawler specializing in Win32, Store Apps, and AppData paths. Uses `walkdir` and `bincode` for sub-millisecond search index loading.
- **PowerShell Integration**: Native bridge for executing .ps1 and .cmd scripts directly from the search bar.
- **Clipboard & I/O Services**: Low-level Rust services for high-speed clipboard polling and injection (via `arboard` or `winapi`).
- **Feedback Engine**: Asynchronous HUD and Toast delivery system using dedicated transparent Tauri overlays.
- **Global Shortcut Listener**: Low-level event listener for the activation hotkey.

### 1.2 The JOR Frontend (Tailwind / Vanilla JS)
- **Rendering Engine**: Optimized DOM manipulation for high-density lists.
- **Design System**: Atomic CSS via Tailwind for instant styling of "Brutalist" components.
- **Event Bus**: Asynchronous communication with the Rust kernel via Tauri's `invoke` API.

## 2. Extension Architecture (Raycast Inspired)
To enable third-party development, JOR is transitioning to an extension-centric model.

### 2.1 Extension Runtime
- **Node.js Sandbox**: Extensions run in a dedicated Node.js process (isolated from the UI).
- **Inter-Process Communication (IPC)**: A JSON-RPC based bridge between the Extension Runtime and the JOR Kernel.

### 2.2 The JOR SDK
- **Declarative UI**: A TypeScript library providing JOR components (List, Item, Form).
- **System APIs**: Controlled access to the filesystem, network, and clipboard.
- **CLI Tooling**: A `jor-cli` for scaffolding, building, and publishing extensions.

## 3. Data & Persistence
- **Search Index**: Binary-serialized cache (`index.cache`) stored in the app data directory.
- **Usage Metrics**: JSON-based storage for ranking and "Most Frequently Used" logic.
- **Configuration**: YAML-based config for user preferences and custom workflows.

## 4. Development Workflow
- **CI/CD**: GitHub Actions for automated building of Windows binaries.
- **Unit Testing**: Rust `#[test]` for kernel logic; Vitest for frontend components.
- **Linting**: `clippy` for Rust; `eslint` for Javascript.
