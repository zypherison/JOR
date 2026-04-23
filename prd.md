# JOR Product Requirements Document (PRD)

## 1. Product Vision
JOR (Just Open & Run) is the fastest, most extensible command palette built specifically for the Windows power user. It provides a "Brutalist" terminal-first experience that prioritizes keyboard-driven workflows and deep integration into the Windows ecosystem (Win32, Store Apps, and WSL).

## 2. Core Objectives
- **Sub-millisecond Speed**: Instant window appearance and real-time search filtering.
- **Extensibility**: Allow developers to build custom commands using a JOR SDK.
- **Consistency**: Maintain a "Cyber-Brutalist" design language across all system and community tools.
- **Reliability**: A robust Rust-based core that manages resources and system indexing efficiently.

## 3. Target Audience
- Developers and Software Engineers.
- DevOps Professionals and System Administrators.
- Power users who prefer terminal-style interaction over GUI-heavy alternatives.

## 4. Key Functional Requirements

### 4.1 Global Command Palette
- **Trigger**: System-wide hotkey (e.g., `Alt + Space`).
- **Input**: Fuzzy-search capable terminal prompt with a blinking block cursor.
- **Navigation**: Full keyboard control (Arrows, Enter, Tab, Esc).

### 4.2 Integrated Indexer (Core Extension)
- Aggressive discovery of Win32 and Store applications.
- Tiered categorization of results (Apps, Folders, Files, System).
- Smart filtering of background binaries and resource files.

### 4.3 Extension Ecosystem & Power APIs
- **Sandbox Environment**: Run custom scripts (Node.js/JS) securely.
- **Power APIs**: JOR must expose the following system-level capabilities to extensions:
  - **Clipboard**: Programmatic Read/Write/Paste/Clear operations.
  - **Feedback**: Non-intrusive HUDs, Toasts for async progress, and Brutalist Alerts for confirmation.
  - **Window Management**: Move, resize, and tile active Windows (specifically optimized for Windows 10/11).
  - **Command Orchestration**: Ability for extensions to launch other commands or update their own metadata (e.g., dynamic subtitles) in real-time.
- **Centralized UI Components**: Extensions must use JOR's "Brutalist" UI components (`List`, `Grid`, `Detail`, `Form`) to ensure visual consistency.

### 4.4 System Monitoring
- Real-time display of CPU, Memory, and System Status in a persistent footer.

## 5. Non-Functional Requirements
- **Resource Efficiency**: Idle memory usage under 50MB.
- **Offline First**: All core search and indexing capabilities must function without an internet connection.
- **Security**: Extension permissions must be explicitly declared and granted.

## 6. Future Roadmap
- **Extension Store**: A centralized registry for community-built JOR commands.
- **AI Integration**: Natural language command processing via local LLMs.
- **PowerShell Integration**: Native execution of complex .ps1 scripts with JOR UI wrappers.
