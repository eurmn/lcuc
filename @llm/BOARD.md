# LCU Companion — Agent Message Board

Use this board for quick inter-agent notes, contract clarifications, caveats, and persistent handoff state across sessions.

---
### [@orchestrator] - Init
- Board initialized. Use append-only entries with `### [@agent] - YYYY-MM-DD HH:MM`.

### [@architect] - 2026-10-29
- **Detached Native Rust Overlay Architecture**:
  - Binary target: `[[bin]] name = "lcu-overlay" path = "src/bin/lcu_overlay.rs"` in `src-tauri/Cargo.toml` (plus default bin `lcu-companion` at `src/main.rs`).
  - Service module: `src-tauri/src/services/native_overlay.rs` (shared IPC structs, single-shot screenshot anchor calibration, BGRA/GDI/PNG renderer, and child-process supervisor).
  - IPC: Parent Tauri process spawns `lcu-overlay.exe` with piped `stdin`/`stdout` JSON lines (`OverlayIpcCommand` / `OverlayIpcEvent`).
  - Rendering: Pure Win32 `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` window driven by `GetMessageW` + `WM_APP + 1` wakeups + `UpdateLayeredWindow` with 32-bit premultiplied BGRA DIB section + GDI font text & cached `image::RgbaImage` PNG blitting. 0% idle CPU/GPU.
  - Single-shot screenshot calibration: Captures game window once when game starts / resolution changes (or on F8 scan), computes & caches HUD and 3-column augment card anchor rects, frees screenshot buffer immediately.
  - WebView2 replacement: Remove `"overlay"` window from `src-tauri/tauri.conf.json` and `src-tauri/src/lib.rs`; route `live_overlay.rs` and `live_game.rs` through `native_overlay`.

### [@surgical-engineer] - 2026-10-29 18:00
- Implemented detached `lcu-overlay` native Win32 layered overlay with stdio JSON lines IPC.
- Configured binary targets in `Cargo.toml`, removed WebView2 overlay config in `tauri.conf.json`.
- Integrated supervisor into `live_overlay.rs`, `live_game.rs`, `champ_select.rs`, `lib.rs`.
- Verified `cargo test --lib native_overlay` (3/3 pass) and `cargo check --bins` (clean).
