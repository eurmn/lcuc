# Task: Native Overlay Extensive Logging & Visibility Diagnostics (`task-overlay-diagnostics-logging`)

## Problem Context
The user started a match but the in-game Win32 native overlay did not show up.
Previously:
1. `lcu-overlay.exe` was not built in `target/debug/` during `bun run tauri dev`.
2. `lcu-overlay` had orphaned process issues on app exit.
3. Errors, panics, Win32 window creation failures, `UpdateLayeredWindow` issues, or League window tracking failures were silently lost because `lcu-overlay` had no structured logging wired into the companion's `logger.rs` / `LogsModal.svelte`.

## Implementation Log
- Extended `OverlayIpcEvent` with `Log { level: String, message: String }`.
- Attached child `stdout` and `stderr` listeners in `src-tauri/src/services/native_overlay.rs` forwarding to `crate::logger::write` (`source: "overlay"` and `source: "native_overlay"`).
- Added child logging inside `run_detached_overlay_process()`:
  - HWND creation handle & errors.
  - Stdin command handling (`CalibrateOnce`, `Render`, `Hide`, `Shutdown`).
  - `UpdateLayeredWindow` results and `GetLastError` reporting.
- Added host logging in `src-tauri/src/services/live_game.rs` and `src-tauri/src/services/live_overlay.rs`:
  - Active match detection with champion name and window rect.
  - Overlay suppression diagnostics (`enabled`, `window_found`, `is_foreground`).
  - Overlay sync and process spawn status.
- Rebuilt all binaries (`cargo build --bins`).
- Verified unit tests (`cargo test --lib native_overlay`: 4/4 passed).
- Verified TypeScript (`type_check`: 0 errors).
- Walkthrough archived at `@llm/docs/historical/walkthrough/overlay_diagnostics_logging_walkthrough.md`.
