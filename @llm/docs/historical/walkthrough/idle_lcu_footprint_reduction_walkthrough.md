# Walkthrough: Idle LCU & Live Client Footprint Reduction

## Problem
During application idle (when out-of-game and not crawling new global ARAM data), the application was issuing continuous and redundant HTTP requests and polling cycles:
1. `src-tauri/src/services/live_game.rs` was calling `GET https://127.0.0.1:2999/liveclientdata/allgamedata` every 1.5 seconds even when League game client wasn't running, while scanning Win32 windows every 75ms.
2. `src/lib/components/history/MayhemChampSelectView.svelte` ran an unconditional `setInterval` every 2.5 seconds calling `getLiveGameState()`, creating a duplicate HTTP request stream.
3. `src-tauri/src/services/lcu_websocket.rs` checked for credentials every 1 second when disconnected, triggering repeated lockfile and `sysinfo` scans, and called `GET /lol-perks/v1/pages` on every gameflow/summoner event even if the companion slot was already verified.
4. `src-tauri/src/agent/mod.rs` collector loop polled `GET /lol-summoner/v1/current-summoner` every 60s despite the summoner PUUID being known, and if the global crawler had reached its game limit, it repeatedly completed and rebuilt SQLite statistics projections on a 60-second infinite loop.
5. `src-tauri/src/lib.rs` system tray status loop queried the agent supervisor every 3 seconds while idle.

## Changes

### 1. Zero Out-of-Game Live Client Requests (`src-tauri/src/services/live_game.rs`)
- When `!window_rect.found`:
  - Skips `fetch_live_game_state()` entirely. Zero requests to `127.0.0.1:2999` when out of game.
  - Clears overlay cards/window on active-to-inactive transition and emits `not_in_game`.
  - Backs off window detection loop sleep to 1500ms.
- When `window_rect.found`:
  - Tracks window at 75ms intervals and polls Live Client API every 20 ticks (1.5s).

### 2. Event-Driven Frontend Live State (`src/lib/components/history/MayhemChampSelectView.svelte`)
- Removed `livePollTimer = setInterval(refreshLiveGameState, 2500)` from `onMount`.
- Refactored `refreshLiveGameState(incomingState?: LiveGameState)` to use pushed payloads directly without querying `getLiveGameState()`.
- Updated `unlistenLiveState` to push incoming live game state or clear advice when out of game.

### 3. Reconnect & Slot Caching (`src-tauri/src/services/lcu_websocket.rs`)
- Changed disconnected reconnect sleep from 1s to 5s.
- Added `COMPANION_SLOT_READY: AtomicBool` so `ensure_companion_slot` only runs once per connection.

### 4. Collector & Crawler Idle Dormancy (`src-tauri/src/agent/mod.rs`)
- Preserves `last_reconciled_puuid` on idle collector ticks without making unnecessary `/lol-summoner/v1/current-summoner` calls.
- Pre-checks `current >= limit` for `global_aram_database_path()` before claiming global collection running, preventing 60-second repeated projection rebuild loops when collection is complete.

### 5. System Tray Backoff (`src-tauri/src/lib.rs`)
- Sleep duration increased to 10s when crawler is idle or paused.

## Verification
- `cargo check --manifest-path src-tauri/Cargo.toml --bins`: Clean build.
- `bun run check`: 0 errors, 0 warnings.
