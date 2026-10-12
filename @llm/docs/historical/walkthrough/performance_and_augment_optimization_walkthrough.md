# Performance, UI & Augment Optimization Walkthrough

## Summary of Changes

### 1. Eliminating Base64 IPC Payload Bloat
- **Issue**: Previously, `enrich_statistics_response` and match history converted every item/augment icon into an inline base64 string (`data:image/png;base64,...`). For 100 rows + associated context items, this caused multi-megabyte payloads to be serialized across Tauri loopback IPC and parsed by JavaScript on every query, causing visible freezing and delays.
- **Fix**: Updated [`services/history.rs`](src-tauri/src/services/history.rs) to use direct CDN URLs (`raw_icon_url`) for item and match history icons. The webview browser handles disk caching and rendering natively with minimal serialization overhead.

### 2. Database Indexes & Query Cache TTL
- **Issue**: Unindexed queries in `agent.db` and `global-aram.db` forced sequential scans when aggregating facts by champion and game ID. Additionally, query results had a very short 5-second TTL in `STATISTICS_QUERY_CACHE`.
- **Fix in [`agent/mod.rs`](src-tauri/src/agent/mod.rs)**:
  - Added indexes on `game_build_facts(champion_id)`, `game_build_facts(game_id)`, `game_build_facts(puuid)`.
  - Added indexes on `aram_participants(champion_id)`, `aram_participants(game_id)`.
  - Increased query cache TTL to 30 seconds (retained up to 120 seconds), allowing smooth sorting and filtering without repeated SQLite query hits.

### 3. Svelte 5 Runes & Reactive Re-render Stabilization
- **ProfileStatisticsView & BuildStatisticsView**:
  - Eliminated conflicting `$effect` + `onMount` loops that caused constant flashing of the "refreshing" indicator.
  - Implemented 200ms debounced search handling (`handleSearchInput`) so keystrokes don't trigger cascading IPC calls.
  - Fixed undeclared `selectedId` state variable in [`ProfileStatisticsView.svelte`](src/lib/components/history/ProfileStatisticsView.svelte).
  - Used `untrack()` for query parameter effects to eliminate cyclic re-triggering.

### 4. Adaptive Background Polling in App.svelte
- **Issue**: [`App.svelte`](src/App.svelte) polled `global_aram_config` on a rigid 1-second `setInterval`, consuming steady CPU and IPC bandwidth even when idle.
- **Fix**: Replaced with an adaptive timeout loop: polls at 1 second only while crawler/projection operations are active, backing off to 6 seconds when idle.

### 5. Augment Resolver Overhaul
- **Issue**: Augments showed raw internal IDs (e.g. `ARAM_Zealot`, `Augment #1316`) or broken icons due to searching only `kiwi.bin.json` and attempting guessed file paths.
- **Fix**:
  - Upgraded [`services/history.rs`](src-tauri/src/services/history.rs) to fetch and parse the official `cherry-augments.json` catalog from CommunityDragon (which contains localized English titles and precise icon paths for Arena and ARAM Mayhem), with `kiwi.bin.json` as a secondary fallback.
  - Multi-keyed the catalog by numeric `id`, `augmentNameId` (`ARAM_Zealot`), stripped prefix (`Zealot`), and lowercase variants.
  - Resolved icons to direct CommunityDragon URLs, eliminating broken/missing icons.
  - Versioned local cache to `augments_v2.json` to invalidate any previously cached raw IDs.

### 6. Collector Loop & Terminal Logging Churn Fix
- **Issue**: Every 15s, `collector_loop` in [`agent/mod.rs`](src-tauri/src/agent/mod.rs) logged `Collector account scan status...` at `INFO` level and ran `refresh_statistics_projections` unconditionally, filling `agent.operations` and triggering the UI banner and table re-renders even when no new games were played.
- **Fix**:
  - Tracked whether `store_recent_games` actually inserted new games (`has_new`).
  - Projection re-calculation is now only spawned when new games were actually found or on explicit user refresh.
  - Reduced repetitive scan status messages from `INFO` to `DEBUG`, preventing terminal spam.

---

## Validation Results

- **Rust Formatting**: `cargo fmt --manifest-path src-tauri\Cargo.toml -- --check` passed cleanly.
- **Svelte 5 Type Check**: `pnpm svelte-check` passed with **0 errors**.
- **Rust Compilation**: `cargo build --manifest-path src-tauri\Cargo.toml --bin lcu-companion-agent` verified.
