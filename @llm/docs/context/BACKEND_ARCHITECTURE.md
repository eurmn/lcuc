# Backend Architecture Direction

## Current implementation

The current product is a single Tauri process. `src-tauri/src/lib.rs` starts `LcuWsWatcher` during Tauri setup, and the watcher runs its reconnect loop on Tauri's async runtime. Tauri commands call Rust services directly. `RuneStore` owns the existing SQLite file for rune pages, favorites, and app settings. Match history, detail caching, catalog lookup, and ARAM Mayhem augment resolution are currently implemented as Tauri services.

This is functional for the existing UI, but it couples long-running LCU/network/database work to the desktop shell and makes background collection dependent on the Tauri process lifecycle.

## Target direction

The application will be split into:

- **Tauri UI process**: Svelte frontend, typed UI commands, window lifecycle, local presentation state, and an authenticated client for the agent.
- **LCU Companion Agent**: separate Rust process/crate responsible for LCU discovery, REST, websocket events, synchronization, collection, catalog/assets, SQLite writes, and statistics projections.
- **Agent-owned SQLite**: one canonical database with versioned migrations and a single-writer rule.

The first compatibility layer should preserve the existing `api_*` command names while routing their implementation to the agent. This allows the frontend to migrate incrementally rather than requiring a UI rewrite and backend split in one change.

## Source of truth

LCU REST responses and websocket events are the external source of truth. Websocket messages trigger targeted work, but reconnect and startup reconciliation must query the relevant endpoints because websocket delivery can be interrupted. The agent database is the durable application source of truth for collected history and normalized metadata.

## Persistence migration

The existing Tauri SQLite store must be backed up and imported into the agent database. The migration must be versioned, idempotent, and produce an import report. Existing runesets, favorite champions, and app settings must be preserved. Tauri should stop opening the canonical database directly after the agent cutover.

The planned normalized schema covers accounts, games, participants, items, runes, augments, raw responses, catalogs/assets, sync cursors, collection errors, and derived statistics. Large icon data remains in the asset cache; relational rows store stable cache keys and metadata.

## Collection model

The agent sleeps while League is unavailable and uses bounded exponential backoff for discovery/reconnect. When connected, it subscribes to `OnJsonApiEvent`, reconciles current state, and schedules post-game history/detail jobs. Game IDs are deduplicated, details are fetched with bounded concurrency, and each completed game emits an incremental update. This is especially important for ARAM Mayhem, where detailed game responses expose augment IDs that summary history may omit.

## Planned API

The local agent API will expose health/state, profile, runesets, matches, statistics, sync commands, and an authenticated event stream. Tauri adapts these DTOs to existing frontend types until the UI can consume the agent contract directly.

## Implementation status

### Implemented in the first separation slice

- Added a standalone `lcu-companion-agent` Rust binary target.
- Added authenticated loopback JSON-lines IPC for health/state requests.
- Added agent-owned SQLite bootstrap at the application data directory.
- Added Tauri supervision and the `api_agent_status` compatibility command.
- Added Tauri startup registration for the agent and a system-tray lifecycle: left-click restores the window, closing the window hides it, and the tray menu provides restore/quit actions.
- Added a development fallback that starts the agent through Cargo when the sibling executable is not present.
- Added agent-side paginated history and detail-cache backfill. On the first connection for an account, the agent requests successive 20-game pages until the LCU returns an empty page or repeats a page, persists summaries, and writes available `/lol-match-history/v1/games/{gameId}` responses into the shared match-detail cache. Later polling only refreshes the newest page while preserving the full discovered range; unavailable details are logged and do not abort the scan.
- Added an agent-side `OnJsonApiEvent` websocket listener with reconnect behavior.
- Kept the existing Tauri LCU watcher and direct services active temporarily; they are not yet cut over.

### Done before the split

- LCU lockfile/process discovery and authenticated REST access.
- LCU websocket subscription and reconnect loop inside Tauri.
- SQLite rune store.
- Match summary/detail collection, persistent detail cache, raw route logging, and batched detail requests.
- Champion, item, rune, and ARAM Mayhem augment metadata/icon resolution.
- Cached match history, pagination, and statistics UI.

### To do

- Create the standalone agent crate/binary and supervisor lifecycle.
- Define and implement authenticated local IPC.
- Add schema migrations and import the current Tauri store.
- Move LCU websocket/REST ownership to the agent.
- Add durable collection jobs, cursors, retry state, and normalized game tables.
- Route existing Tauri commands through the agent.
- Add automatic post-game collection and incremental event delivery.
- Add agent-owned statistics projections and health/observability.
- Evaluate an optional, isolated Pengu Loader bridge after the core architecture is stable.
