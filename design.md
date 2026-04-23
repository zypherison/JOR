# JOR Design Principles & System

## 1. Aesthetic Identity: Cyber-Brutalist
JOR is a Windows-exclusive tool that rejects the soft, rounded "friendly" UI of modern OSs in favor of a sharp, high-density terminal aesthetic. It is built to feel like a "Power Console" for the Windows environment, appealing to users who need a bridge between the command line and application launching.

### 1.1 Visual Anchors
- **Sharp Edges**: `0px` border radius on all primary containers.
- **High Contrast**: Pure black backgrounds (`#0e0e0e`) paired with high-frequency cyan accents (`#96f8ff`).
- **Terminal Symbols**: Heavy use of monospace fonts and terminal indicators (e.g., `$`, `_`, block cursors).
- **Grid Patterns**: Subtle background radial gradients that simulate a terminal grid.

## 2. Component Architecture
To ensure extensions look native to JOR, the platform provides a restricted set of declarative components. Developers do not write custom CSS; they assemble UI using JOR's "Brutalist" building blocks.

### 2.1 The List & Grid Components
- **List**: Standard high-density row layout for apps and files.
- **Grid**: Optimized for visual assets (Icons, Images). Uses a stark, bordered box model with zero-margin thumbnails.
- **Header**: Contains the Process_Queue or Path_Mapper category name.
- **Search Bar**: A unified input with a blinking block cursor and keyboard hints (`ENTER`, `CMD+K`).
- **List/Grid Item**: 
  - **Badge**: Sharp-edged indicator for type (APP, DIR, FILE, CMD).
  - **Metadata**: Monospace path or subtitle.
  - **Action Hint**: Clear instruction on what the `Enter` key will do.

### 2.2 Feedback Modals (The Brutalist Toast)
- **Toast**: Small, sharp-edged rectangles anchored to the bottom-right. Features a success-cyan or error-red border.
- **HUD (Heads-Up Display)**: Large, centered, semi-transparent overlays for quick confirmation (e.g., "COPIED"). Uses massive uppercase typography.
- **Alert**: Full-screen modal blur with a centered, high-contrast box for destructive actions. Requires explicit confirmation (e.g., `HOLD_ENTER_TO_DELETE`).

### 2.3 The Detail View & Forms
- **Detail View**: Triggered by `Tab` on a selected item. Displays comprehensive metadata in a split-pane layout.
- **Form Component**: Stark, high-contrast input fields with zero-padding for structured data entry.

## 3. Interaction Logic
- **Speed over Fluidity**: Animations are extremely fast (sub-100ms) and use "snappy" easing functions.
- **Visual Feedback**: Selection is indicated by inverting the colors (Black on Cyan) rather than a subtle highlight.
- **System Pulse**: The footer provides a constant "heartbeat" of system health via the Success Pulse animation.

## 4. Typography
- **Headline**: `Space Grotesk` (Bold, geometric).
- **Body**: `Inter` (Clear, readable).
- **System/Data**: `JetBrains Mono` (Technical precision).
