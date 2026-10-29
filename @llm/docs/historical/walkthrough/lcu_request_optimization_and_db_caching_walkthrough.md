# LCU Request Optimization, Persistent Database Caching, and Topbar Signal Walkthrough

## Summary of Changes

### 1. LCU Request Storm Elimination & Throttling
- **WebSocket Event Loop Isolation** (`src-tauri/src/agent/mod.rs`):
  - Removed match-history URI trigger (`uri.contains("match-history")`) in `websocket_loop`. Eliminated self-inflicted feedback storm where collector queries fired LCU match-history events that immediately re-triggered the collector.
- **Collector Loop Cooldown & PUUID Caching** (`src-tauri/src/agent/mod.rs`):
  - Added 45-second cooldown on personal match history reconciliation.
  - Cached current summoner PUUID for 30 seconds to avoid repeating `/lol-summoner/v1/current-summoner` calls every loop iteration.
- **Patch Freshness Check Throttling** (`src-tauri/src/services/patch_manager.rs`):
  - Enforced 5-minute (300-second) cooldown on `ensure_patch_freshness` calls even when `LcuClient` is passed, avoiding spamming `/lol-patch/v1/game-version` and `/system/v1/builds`.
- **Process Scan Cooldown** (`src-tauri/src/services/lcu_client.rs`):
  - Added 5-second rate limiting on full process scans (`sysinfo`) when standard lockfiles are absent.
- **Paced Match History Fetching** (`src-tauri/src/services/history.rs`):
  - Replaced unthrottled concurrent `join_all` batch calls with sequential 75ms paced detail requests to protect the LCU process from timeouts and socket exhaustion.

### 2. Match Detail Multi-Tier Cache & No-Refetch Guarantee
- **Unification with SQLite `aram_games`** (`src-tauri/src/services/history.rs`):
  - `read_detail_cache` now checks disk cache (`cache/match-history/{id}.json`), and if missing, queries `aram_games.detail_json` in `global-aram.db`.
  - When hit from SQLite, automatically saves to disk cache so the game is permanently cached on disk and never fetched again from LCU.
- **Global Crawl Detail Storage** (`src-tauri/src/agent/mod.rs`):
  - `cache_page_details` now checks whether a match ID already exists in `aram_games` before fetching from LCU.
  - `store_global_detail` writes the crawled match JSON directly to disk cache (`cache_game_detail`).

### 3. Persistent Database Caching of Computed Statistics
- **Removed Periodic Projection Wipe** (`src-tauri/src/agent/mod.rs`):
  - Deleted 60-second invalidation loop (`LAST_PROJECTION_INVALIDATION` with `DELETE FROM statistics_projections`) inside `store_global_detail`. Projections persist and are updated in-place via `refresh_statistics_projections`.
- **Extended In-Memory Caches**:
  - `STATISTICS_QUERY_CACHE` TTL increased from 60s to 600s (10 minutes).
  - `ENRICHED_DATASET_CACHE` TTL increased from 60s to 600s (10 minutes).

### 4. Background Progress Visibility & Signal Component
- **Proactive Step Reporting** (`src-tauri/src/agent/mod.rs`):
  - `refresh_statistics_projections` now updates `operation.message` and `operation.current` *before* starting each calculation phase ("Computing champion statistics (1/4)", "Mining synergy combos (4/4)").
- **Topbar Signal Component** (`src/lib/components/TitleBar.svelte`):
  - Replaced 2.5px invisible pulsing dot with an informative operations pill displaying active task description, live pulse indicator, step fraction `(current/total)`, and badge for additional tasks.
  - Hover dropdown displays multi-task progress bars and stage names.
- **Top Progress Banner** (`src/App.svelte`):
  - Expanded progress banner to display active calculation operations and stages even when the crawler is not running, eliminating blank loading screens.
