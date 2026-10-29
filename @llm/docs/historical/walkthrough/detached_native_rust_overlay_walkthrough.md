# Walkthrough: Detached Native Rust Overlay Service & Architecture

## Overview
Replaced the Tauri WebView2 HTML-based in-game overlay with a fully detached, lightweight Win32 Rust process (`lcu-overlay`) that renders simple PNG sprites and text directly into an unmanaged layered window.

## Changes

### 1. Detached Process & Build Targets
- **`src-tauri/Cargo.toml`**: Added binary targets `lcu-companion` (main app) and `lcu-overlay` (`src/bin/lcu_overlay.rs`). Added `Win32_System_LibraryLoader` to `windows-sys`.
- **`package.json`**, **`src-tauri/Cargo.toml`**, **`src-tauri/tauri.conf.json`**: Bumped version `0.1.1` -> `0.1.2`.
- **`src-tauri/tauri.conf.json`**: Removed the secondary `"overlay"` WebView2 window definition.

### 2. Native Overlay Service & Win32 Layered Window
- **`src-tauri/src/bin/lcu_overlay.rs`**: Entry point for detached child process, running `run_detached_overlay_process()`.
- **`src-tauri/src/services/native_overlay.rs`**:
  - Implemented newline-delimited JSON IPC (`OverlayIpcCommand` / `OverlayIpcEvent`) over stdin/stdout.
  - Implemented single-shot screenshot calibration (`calibrate_layout_from_single_screenshot`) taking one GDI capture to calculate HUD pill and 3-column card anchors, releasing capture buffers immediately.
  - Implemented event-driven Win32 layered popup window (`WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`) updated via `UpdateLayeredWindow` with preallocated 32-bit premultiplied BGRA DIB section.
  - Implemented scratch-DIB luminance-to-alpha font renderer for crisp opaque text and PNG sprite alpha-blending with caching.
  - Implemented `NativeOverlaySupervisor` managing child process lifecycle, auto-restart, and calibration caching.

### 3. Integration with Live Game & Commands
- **`src-tauri/src/services/live_overlay.rs`**: Redirected window sync, state updates, and visibility to `NativeOverlaySupervisor`.
- **`src-tauri/src/services/live_game.rs`**: Pushes detected window rects, game state, and scanned augment cards to `NativeOverlaySupervisor`.
- **`src-tauri/src/commands/champ_select.rs`**: Wired `api_sync_overlay_window`, `api_scan_live_augment_cards`, and `api_emit_overlay_state` to supervisor.
- **`src-tauri/src/lib.rs`**: Initialized `NativeOverlaySupervisor` during app setup; removed WebView2 overlay styling.

## Verification
- Unit tests (`cargo test --lib native_overlay`):
  - IPC command/event serialization roundtrip: passed.
  - 1080p and 1440p anchor layout calculations: passed.
  - Premultiplied BGRA rect & sprite blitting: passed.
