### Walkthrough: Native Overlay Diagnostics & Extensive Logging

#### Problem
In live matches, the detached Win32 native overlay (`lcu-overlay.exe`) did not appear, and there was no visibility into child process state, calibration events, window handle allocation, or rendering failures.

#### Solution
1. **Bi-Directional Child Logging**:
   - Added `OverlayIpcEvent::Log { level, message }` to the IPC contract.
   - Wired child `stdout` and `stderr` capture threads in `services/native_overlay.rs` into `crate::logger::write`.
   - All events and console output from `lcu-overlay` now stream directly into the frontend Logs Modal (`Ctrl + L` or Titlebar terminal button) with `source: "overlay"`.
2. **Comprehensive Overlay Diagnostic Instrumentation**:
   - `lcu-overlay`: Emits logs on process start, window class registration, HWND creation, calibration receipt, render calls, and `UpdateLayeredWindow` Win32 API results (including `GetLastError()` if failed).
   - `services/live_game.rs`: Added structured logging when League window is detected, when active match starts, when in-game polling succeeds or fails, and when overlay is suppressed due to foreground/enabled state.
   - `services/live_overlay.rs`: Added logging on `sync_overlay_window` and error logging on process spawn failure.
3. **Target Binaries Synchronized**:
   - Compiled `lcu-overlay` and `lcu-companion` in `target/debug/` with `cargo build --bins`.
   - Verified that both binary targets are updated and unit tests pass.

#### Verification
- `cargo test --manifest-path src-tauri/Cargo.toml --lib native_overlay`: 4/4 passed.
- `cargo build --manifest-path src-tauri/Cargo.toml --bins`: Finished clean dev profile.
- `type_check`: 0 TypeScript errors.
