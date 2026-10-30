# Sub-Board: task-lcu-idle-footprint

## Goal
Reduce LCU request footprint and background polling frequency when the application is idle (not crawling global ARAM and not in-game). Ensure the app runs with near-zero overhead and minimal LCU HTTP queries when idle.

## Scope & Constraints
1. Investigate all periodic background tasks in Rust (Tokio loops, collector, supervisor, websocket watcher, live game watcher, overlay polling).
2. Investigate all frontend intervals and polling effects (in Svelte components: `App.svelte`, `MayhemChampSelectView.svelte`, `LiveMayhemOverlay.svelte`, etc.).
3. Back off polling intervals dynamically:
   - When LCU is disconnected: slow backoff instead of hammering reconnect.
   - When in idle client (out of game, out of champ-select): rely on LCU WebSocket events rather than high-frequency HTTP polling, or use long backoff intervals (e.g. 5s-15s instead of sub-second or 1s).
   - Only switch to active polling (or rely on game client / live game API) when game is active or champ select event fires.
   - Global ARAM crawler must strictly pause / stay dormant when not actively running, with 0 idle requests.
4. Minimal footprint, no regression on responsiveness when entering champ select or game.

## Progress Log
### [@orchestrator] 2026-10-29 Task Initialized
- Initialized sub-board and registered in `@llm/BOARD.md`.
- Spawning `@scout` to audit all LCU polling, HTTP calls, and frontend intervals.

### [@scout-recon] 2026-10-29 Recon Report: Idle LCU HTTP, Live Client API & Polling Footprint

#### 1. Live Client API (`https://127.0.0.1:2999`) Idle Polling
- **Backend Native Poller**: `src-tauri/src/services/live_game.rs:202-260` (`start_live_game_polling`)
  - **Trigger**: App startup (`src-tauri/src/lib.rs:177`).
  - **Current Frequency**: Outer loop `tokio::time::sleep(Duration::from_millis(75))`. Calls Win32 `FindWindowW` for League Client window every 75ms.
  - **HTTP Request**: Every 20 ticks (1.5s), calls `fetch_live_game_state()`, creating a new `reqwest::Client` (1.5s timeout) sending `GET https://127.0.0.1:2999/liveclientdata/allgamedata`.
  - **Idle Behavior**: Runs continuously every 1.5s indefinitely, even when the user is out of game, in lobby, or League is closed.
- **Frontend Duplicate Poller**: `src/lib/components/history/MayhemChampSelectView.svelte:282`, `382-423`
  - **Trigger**: Mounted view.
  - **Current Frequency**: `livePollTimer = setInterval(refreshLiveGameState, 2500)` (2.5s).
  - **HTTP Request**: Invokes Tauri command `api_get_live_game_state` -> `fetch_live_game_state()` -> duplicate `GET https://127.0.0.1:2999/liveclientdata/allgamedata`.
  - **Idle Behavior**: Directly duplicates backend polling while the view already listens to `live:game-state` push events (line 306).

#### 2. LCU WebSocket Watchers & Disconnect Polling
- **Duplicate WebSocket Watchers**:
  1. `src-tauri/src/services/lcu_websocket.rs:39-60` (`LcuWsWatcher::run_loop` spawned in `lib.rs:176`)
  2. `src-tauri/src/agent/mod.rs:5357-5456` (`websocket_loop` spawned in `agent_supervisor.rs:40`)
- **Disconnect Reconnect / Process Scan Hammering**:
  - `services/lcu_websocket.rs:59`: When disconnected, sleeps only `1s` (`sleep(Duration::from_secs(1))`) and calls `LcuClient::find_credentials()`.
  - `agent/mod.rs:5360`: When disconnected, sleeps `5s`.
  - `services/lcu_client.rs:184-201`: `find_credentials()` checks 6 disk lockfiles, then every 5s runs `sysinfo::System` with `ProcessesToUpdate::All` and `UpdateKind::Always` (CPU/IO spike).
- **Slot Maintenance HTTP Calls**:
  - `services/lcu_websocket.rs:114-116`, `148-155`: On connect and on every `/lol-login/v1/session`, `/lol-summoner/v1/current-summoner`, `/lol-gameflow/v1/gameflow-phase` event, calls `ensure_companion_slot`, making HTTP `GET /lol-perks/v1/pages`.

#### 3. Collector Loop Idle HTTP & Projections Spin
- **File**: `src-tauri/src/agent/mod.rs:5087-5355` (`collector_loop`)
- **Current Frequency**: Loop timer `tokio::select!` with `sleep(Duration::from_secs(60))` or `COLLECTOR_WAKEUP.notified()`.
- **Idle Behavior**:
  - Line 5184: Every 60s tick, `last_summoner_check.elapsed() >= 30s` triggers `GET /lol-summoner/v1/current-summoner`.
  - Lines 5288-5327: When global crawler finishes or reaches game limit, `state.global_collection_running` is reset to `false`. On the next 60s iteration, `should_start` becomes `true` again, spawning `collect_global_aram(...)`, which immediately exits (limit reached) and triggers `refresh_statistics_projections(...)`.
  - Lines 4731-4816: Recomputes/verifies SQLite projections across all 7 scopes/kinds every 60s while idle.

#### 4. Crawler Worker Idle Busy Loop When Paused
- **File**: `src-tauri/src/agent/mod.rs:4013-4016` (`crawler_worker_task`)
- **Current Frequency**: `tokio::time::sleep(Duration::from_millis(1000)).await` (1s).
- **Idle Behavior**: When `crawler_paused` is true, all active worker tasks (default 3, up to 16) stay in busy loops waking up every 1000ms checking the RwLock.

#### 5. Frontend & Tray Status Intervals (Non-LCU, SQLite / Supervisor Polling)
- **Frontend Progress Banner**: `src/App.svelte:143-182` (`refreshGlobalStatus`)
  - Calls `api_global_aram_config` every 6000ms (6s) when idle (`hasActiveWork ? 1000 : 6000`).
- **System Tray Status**: `src-tauri/src/lib.rs:135-173`
  - Spawns background task polling `supervisor.request("global_aram_config")` every 3000ms (3s) to update tray text.

### [@worker-svelte] 2026-10-29 00:00
- Modified `src/lib/components/history/MayhemChampSelectView.svelte`:
  - Removed `livePollTimer = setInterval(refreshLiveGameState, 2500)` from `onMount`.
  - Updated `refreshLiveGameState(incomingState?: LiveGameState)` to use incoming payload directly and avoid calling `getLiveGameState()` when not in game.
  - Updated `live:game-state` event listener to pass active state directly into `refreshLiveGameState(state)` and cleanly clear `liveAdvice`, `buildEngineRec`, and sync overlay on out-of-game.
- Verified with `bun run check`: 0 errors, 0 warnings.

### [@worker-rust] 2026-10-29 00:05
- Modified `src-tauri/src/services/live_game.rs`:
  - When `!window_rect.found`: skips calling `fetch_live_game_state()` entirely (zero requests to 127.0.0.1:2999 out of game), clears overlay state on transition, and backs off loop sleep to 1500ms.
  - When `window_rect.found`: loops at 75ms and polls Live Client API every 20 ticks (1.5s).
- Modified `src-tauri/src/services/lcu_websocket.rs`:
  - Backed off disconnected reconnect sleep from 1s to 5s.
  - Added `COMPANION_SLOT_READY: AtomicBool` to prevent repeated `/lol-perks` lookups on every phase and summoner event once companion slot is verified. Resets to `false` on disconnect.
- Modified `src-tauri/src/agent/mod.rs`:
  - In `collector_loop`: reuses `last_reconciled_puuid` without polling `/lol-summoner/v1/current-summoner` on idle ticks.
  - Pre-checks `current >= limit` for `global_aram_database_path()` before claiming global collection running, preventing 60s re-trigger loop and repeated projection rebuilds when at capacity.
- Modified `src-tauri/src/lib.rs`:
  - Tray status polling sleeps 10s when crawler is idle or paused; 3s when actively crawling.
- Verified: `cargo check --manifest-path src-tauri/Cargo.toml --bins` (clean).

