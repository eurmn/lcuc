# LCU Companion — Agent Message Board

Use this board for quick inter-agent notes, contract clarifications, caveats, and persistent handoff state across sessions.

---
### [@orchestrator] 2026-10-30 12:45 [task-overlay-diagnostics-logging]
- Initialized Task `task-overlay-diagnostics-logging`.
- Sub-board: `@llm/board/task-overlay-diagnostics-logging/board.md`.
- Goal: Implement comprehensive logging across `lcu-overlay` and backend overlay supervisor, stream logs into `logger.rs` / `LogsModal.svelte`, and diagnose in-game overlay rendering.


### [@architect] 2026-10-29
**Detached Native Rust Overlay Architecture**:
- Binary target: `[[bin]] name = "lcu-overlay" path = "src/bin/lcu_overlay.rs"` in `src-tauri/Cargo.toml` (plus default bin `lcu-companion` at `src/main.rs`).
- Service module: `src-tauri/src/services/native_overlay.rs` (shared IPC structs, single-shot screenshot anchor calibration, BGRA/GDI/PNG renderer, and child-process supervisor).
- IPC: Parent Tauri process spawns `lcu-overlay.exe` with piped `stdin`/`stdout` JSON lines (`OverlayIpcCommand` / `OverlayIpcEvent`).
- Rendering: Pure Win32 `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` window driven by `GetMessageW` + `WM_APP + 1` wakeups + `UpdateLayeredWindow` 32-bit premultiplied BGRA DIB section + GDI font text & cached `image::RgbaImage` PNG blitting. 0% idle CPU/GPU.
- Single-shot screenshot calibration: Captures game window once when game starts / resolution changes (or on F8 scan), computes & caches HUD and 3-column augment card anchor rects, frees screenshot buffer immediately.
- WebView2 replacement: Remove `"overlay"` window from `src-tauri/tauri.conf.json` and `src-tauri/src/lib.rs`; route `live_overlay.rs` and `live_game.rs` events to supervisor.

### [@surgical-engineer] 2026-10-29 18:00
- Implemented `lcu-overlay` binary and `native_overlay.rs` with Win32 layered DIB and JSON IPC.
- Configured binary targets in `Cargo.toml`, removed WebView2 overlay config in `tauri.conf.json`.
- Integrated supervisor into `live_overlay.rs`, `live_game.rs`, `champ_select.rs`, `lib.rs`.
- Verified `cargo test --lib native_overlay` (3/3 pass) and `cargo check --bins` (clean).

### [@worker] 2026-10-29 18:30 [task-mayhem-build-engine]
**Modularize Mayhem Build Creation Engine Backend into Independent Subprocess**
- **Sub-board**: `@llm/board/task-mayhem-build-engine/board.md`
- **Walkthrough**: `@llm/docs/historical/walkthrough/build_engine_modularization_walkthrough.md`
- Implemented modular ARAM Mayhem build recommendation engine in `src-tauri/src/build_engine/`.
  - `models.rs`: Typed JSON IPC models (`BuildEngineRequest`, `BuildEngineResponse`, `BuildRecommendationResult`, `ActiveComboSynergy`, `ItemRecommendation`, `EnginePerformanceStats`).
  - `data_loader.rs`: Participant match loader from `global-aram.db` (`aram_participants`) with fallback to `agent.db` (`game_build_facts`).
  - `bitset_miner.rs`: Bitset intersection engine using `Vec<u64>` (64 transactions/word, bitwise `&`, `.count_ones()`). Matches active item-item and item-augment combos, recommends boots (`is_boot_item_id`) or returns `None` if owned, ranks next 2 items using Bayesian shrinkage win rate and synergy lift, and completes 6-item builds. Includes unit test self-check.
  - `cache.rs`: In-memory thread-safe `ChampionTransactionIndex` cache with 5-minute TTL for <1ms query responses.
  - `ipc.rs`: Child-process stdio JSON line reader/writer loop.
  - `host.rs`: Supervisor running in Tauri parent process. Spawns `lcu-build-engine.exe` (`CREATE_NO_WINDOW`), stdin writer, background stdout reader thread oneshot dispatch and 1.5s timeout, plus transparent in-process fallback.
- Binary target: Added `[[bin]] name = "lcu-build-engine" path = "src/bin/lcu_build_engine.rs"`.
- Tauri IPC command: Added `api_compute_mayhem_build_recommendation` in `commands/build_engine.rs`, registered in `commands/mod.rs` and `lib.rs` handlers.
- Process supervisor lifecycle: `ensure_build_engine_process_started()` registered in app setup.
- Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib build_engine::bitset_miner::tests` passed (1 passed), `cargo check --manifest-path src-tauri/Cargo.toml --bins` clean.

### [@orchestrator] 2026-10-29 19:00 [task-mayhem-build-overlay-presentation]
**Frontend Mayhem Build Engine Presentation & Native Overlay CDragon PNG Sprite Rendering**
- **Sub-board**: `@llm/board/task-mayhem-build-overlay-presentation/board.md`
- **Walkthrough**: `@llm/docs/historical/walkthrough/mayhem_build_engine_overlay_presentation_walkthrough.md`
- Completed frontend & native overlay integration for the modular build recommendation engine:
  - Frontend (`src/lib/types/champ_select.ts`, `src/lib/api/champ_select.ts`, `src/lib/components/history/MayhemChampSelectView.svelte`):
    - Added TypeScript DTOs and `computeMayhemBuildRecommendation` wrapper.
    - Added reactive build engine advice card with CommunityDragon 2D item PNG icons, active combos, lift, win rates, and engine latency stats.
    - Synchronized live inventory and simulated items into overlay state payloads.
  - Native Overlay & Asset Caching (`src-tauri/src/services/live_overlay.rs`, `src-tauri/src/services/native_overlay.rs`):
    - Added async CDragon item icon fetcher and cache to `%LOCALAPPDATA%/lcu-companion/cache/cdragon/items/{id}.png`.
    - Generated `OverlayPngSprite` tiles positioned below the HUD banner.
    - Rendered semi-transparent "RECOMMENDED ITEMS" HUD box with role-based tile borders and direct BGRA PNG sprite blitting.

### [@orchestrator] 2026-10-29 19:30 [task-lcu-idle-footprint]
**Reduce LCU Request Footprint & Polling on Application Idle**
- **Sub-board**: `@llm/board/task-lcu-idle-footprint/board.md`
- Completed:
  - Eliminated out-of-game Live Client API polling in `live_game.rs` (0 HTTP requests when game window not found; backed off window scan to 1500ms).
  - Eliminated redundant 2.5s polling interval in frontend `MayhemChampSelectView.svelte`; now purely event-driven from backend `live:game-state`.
  - Backed off LCU disconnected WebSocket loop from 1s to 5s.
  - Cached companion slot readiness in `lcu_websocket.rs` to stop repetitive `/lol-perks` queries on summoner/phase changes.
  - Stopped 60s idle collector summoner polling in `agent/mod.rs` and prevented dormant global crawler from triggering repeated projection rebuilds when game limits are reached.
  - Backed off system tray status polling to 10s when idle in `src-tauri/src/lib.rs`.

### [@orchestrator] 2026-10-29 20:00 [task-mayhem-build-engine-main-ui]
**Clean Legacy Build Advice & Build Dedicated UI for Mayhem Build Engine**
- **Sub-board**: `@llm/board/task-mayhem-build-engine-main-ui/board.md`
- Dispatched 2 collaborative frontend agents (`worker-frontend-architect` and `worker-frontend-data`) to clean out legacy advice and modularize the main window UI for the newly calculated build engine data.

### [@orchestrator] 2026-10-30 08:30 [task-mayhem-build-engine-ui-polish]
**Mayhem Build Engine UI Layout Polish, 5-Item Card Metrics, and Alternative Items Sibling Component**
- **Sub-board**: `@llm/board/task-mayhem-build-engine-ui-polish/board.md`
- Polished 5-item build path with `h-11 w-11` icons, WR, games, and recommendation justification.
- Extended `BuildRecommendationResult` with `completed_build: Vec<ItemRecommendation>` and `alternative_items: Vec<ItemRecommendation>`.
- Added dedicated alternative items section inside the Build Engine panel displaying empirical metrics and algorithm justifications without the word "why".
- Verified: `cargo test --lib build_engine` (2 passed), `cargo check --bins` (clean), `type_check` (0 errors).

### [@orchestrator] 2026-10-30 11:00 [task-overlay-polish-aug-synergies]
**Overlay Recommended Sprites, OCR Reliability, and Augment Synergies**
- **Sub-board**: `@llm/board/task-overlay-polish-aug-synergies/board.md`
- Resolving missing item sprites in native overlay via byte embedding in IPC.
- Rewriting augment card synergy enrichment with empirical bitset co-occurrence for 1-2 commonly bought legendary items.
- Widening OCR title band, allowing 1+ card detections, and displaying on-screen error/status feedback.







