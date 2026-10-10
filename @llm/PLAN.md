# LCU Companion Backend Separation and Statistics Platform Plan

## Status

The first implementation slice is now in progress. A standalone agent binary, authenticated loopback IPC, agent-owned SQLite bootstrap, legacy rune-store import foundation, Tauri supervision, and basic LCU summary/websocket collection have been added. The full cutover remains incomplete.

The application now uses Fontsource Nunito Sans globally for a friendly, readable visual voice. Match-history cards emphasize game-analysis context: result, champion, mode, KDA, CS, gold, optional champion damage, map, duration/date, and augments.

The current application remains a Tauri 2 desktop application with a Svelte 5 frontend and Rust services running inside the Tauri process. This plan defines the migration to a separately running data/LCU agent without breaking the existing rune-builder UI.

## Goals

1. Move LCU discovery, REST access, websocket subscriptions, synchronization, post-game collection, catalog resolution, and statistics aggregation out of the Tauri UI process.
2. Make the separate agent the owner of the long-lived application database.
3. Keep Tauri focused on presentation, user commands, local UI preferences, and communication with the agent.
4. Continue working when League is closed: the agent sleeps, preserves state, and reconnects when the client becomes available.
5. Automatically collect completed games from the LCU, with special support for ARAM Mayhem augment, item, damage, and timeline data.
6. Preserve existing rune recommendations, saved runesets, client synchronization, and cached history during migration.
7. Expose a stable local API so the frontend is not coupled to Rust service internals.
8. Keep Pengu Loader integration optional and isolated from the core collector.

## Non-goals

- Do not inject code into the League client as part of the core application.
- Do not make Pengu Loader a required dependency.
- Do not delete or rewrite existing user data without an explicit migration and backup path.
- Do not assume every LCU endpoint is stable across client versions; record endpoint capability and failure state.

## Target architecture

```text
                         ┌─────────────────────────┐
                         │ Svelte 5 UI              │
                         │ Tauri commands/events    │
                         └────────────┬────────────┘
                                      │ local IPC
                         ┌────────────▼────────────┐
                         │ Tauri UI adapter         │
                         │ lifecycle + proxy only   │
                         └────────────┬────────────┘
                                      │
                         ┌────────────▼────────────┐
                         │ lcu-companion-agent      │
                         │ standalone Rust process  │
                         ├─────────────────────────┤
                         │ Supervisor               │
                         │ LCU discovery            │
                         │ REST client              │
                         │ Websocket event reader   │
                         │ Sync/state coordinator   │
                         │ Post-game collector      │
                         │ Catalog/cache workers    │
                         │ Statistics projector     │
                         │ Local agent API          │
                         └────────────┬────────────┘
                                      │ single writer
                         ┌────────────▼────────────┐
                         │ lcu-companion SQLite     │
                         │ games, participants,     │
                         │ augments, items, runes,  │
                         │ sync cursors, settings   │
                         └────────────┬────────────┘
                                      │
                       ┌──────────────▼──────────────┐
                       │ League Client Update (LCU)  │
                       │ REST + OnJsonApiEvent WAMP  │
                       └─────────────────────────────┘
```

### Process responsibilities

#### Standalone agent

- Discover the LCU lockfile/process and maintain credentials.
- Sleep with a bounded retry schedule while League is unavailable.
- Own the LCU websocket connection and subscribe to `OnJsonApiEvent`.
- Reconcile state after connect, reconnect, login, logout, gameflow changes, and missed events.
- Collect match summaries and `/lol-match-history/v1/games/{gameId}` details.
- Deduplicate games by the stable LCU game ID.
- Resolve champion, item, rune, and ARAM Mayhem augment metadata.
- Persist raw responses where useful for diagnostics and deterministic reprocessing.
- Project normalized records and aggregate statistics into SQLite.
- Provide read and command endpoints to Tauri.
- Emit incremental updates so the UI can render cached data immediately and refresh progressively.

#### Tauri process

- Start, stop, and monitor the agent.
- Connect to the agent over a local authenticated IPC transport.
- Adapt agent responses to existing typed Tauri commands while the frontend migration is incremental.
- Render the existing rune builder and profile/statistics UI.
- Keep only UI-local state such as selected tabs, window state, and temporary form state.
- Avoid direct LCU requests once the agent is authoritative.

### Recommended transport

Implement the agent as a sibling Rust binary/crate in the same repository. Start it from Tauri with a generated per-session authentication secret and a local loopback transport. Prefer a local named pipe on Windows; retain a loopback TCP fallback only if the chosen IPC library requires it. The agent must reject unauthenticated clients and bind only to the local machine.

The first API should be request/response plus server-sent event notifications:

- `GET /health`
- `GET /state`
- `GET /profile`
- `GET /runesets`
- `GET /matches`
- `GET /statistics/champions`
- `GET /statistics/champions/{champion_id}`
- `POST /runesets/import`
- `POST /runesets/export`
- `POST /sync`
- `POST /shutdown`
- event stream: `connected`, `disconnected`, `sync_started`, `match_discovered`, `match_enriched`, `projection_updated`, `sync_error`

The exact transport may change during implementation, but the domain API and event names should remain stable.

## Database ownership and migration

### Canonical database

Create one agent-owned SQLite database under the existing application data directory, using a versioned schema migration table. The agent is the sole writer. Tauri reads through the agent API rather than opening the database directly.

Retain the current rune database file as an input during migration. On first agent start:

1. Acquire an application migration lock.
2. Back up the existing database before mutation.
3. Detect the existing schema and import `runesets`, `favorite_champions`, and `app_settings`.
4. Preserve original IDs where safe; otherwise record `legacy_source` and `legacy_id`.
5. Resolve duplicate runesets deterministically.
6. Mark the migration version and import report.
7. Make the migration idempotent.

If another Tauri store is discovered, import it through a separate adapter rather than silently merging arbitrary SQLite files.

### Initial schema areas

- `schema_migrations`
- `accounts` keyed by PUUID
- `summoners` and profile snapshots
- `runesets`, `runeset_perks`, `favorite_champions`, `app_settings`
- `games` with LCU IDs, queue/map/mode, timestamps, duration, version, and collection status
- `game_participants` with player identity, champion, result, KDA, gold, CS, damage, vision, and role
- `game_items` with slot, item ID, resolved name, icon key, and source
- `game_runes` with primary/sub styles, perk IDs, and stat shards
- `game_augments` with slot, augment ID, resolved name, icon key, and catalog source
- `game_teams`, bans, and objective summaries
- `raw_lcu_responses` with route, game ID, response hash, payload path/blob reference, and collected timestamp
- `catalog_entries` and `asset_entries` with URL, content hash, state, and retry metadata
- `sync_cursors` for summary pages, detail collection, and last successful reconciliation
- `collection_errors` for retryable and terminal failures
- derived statistic tables or materialized views introduced only after normalized data is stable

Use foreign keys, indexes on account/game/time/champion/queue, WAL mode, busy timeout, and bounded transactions. Do not store large binary icons in the relational tables; keep them in the existing asset cache and store stable cache keys/metadata.

## Synchronization design

### Connection lifecycle

The supervisor has explicit states:

`Sleeping` -> `Discovering` -> `Connecting` -> `Reconciling` -> `Live` -> `Backoff`.

- `Sleeping`: no LCU credentials found; no tight loop.
- `Connecting`: build the authenticated REST and websocket clients.
- `Reconciling`: fetch current summoner, gameflow, rune pages, recent history, and pending detail jobs.
- `Live`: consume websocket events and schedule targeted reads.
- `Backoff`: retry with exponential backoff and jitter after disconnect or errors.

Websocket events are triggers, not the sole source of truth. On every connect/reconnect and on relevant gameflow transitions, run a reconciliation pass to recover from missed events.

### Important event triggers

- LCU login/session changes
- current summoner changes
- gameflow phase changes
- match history changes
- post-game/statistics changes
- rune page/current page changes
- websocket disconnect/reconnect

Keep endpoint subscriptions and event-to-job mappings in a registry so client-version changes are isolated.

### Post-game collection

When the gameflow indicates post-game or the history endpoint changes:

1. Read the current summoner identity.
2. Fetch recent summary pages using a cursor/window.
3. Upsert unseen games.
4. Queue missing detail records.
5. Fetch details with a bounded concurrency limit (initially 10).
6. Extract participant statistics, items, runes, and `playerAugment1..6`.
7. Resolve metadata through persistent catalog/asset caches.
8. Mark the game complete only after required data has been persisted.
9. Rebuild affected statistic projections.
10. Emit incremental events for each game and projection update.

A game detail request is immutable after successful collection for normal history purposes, but retain a manual re-fetch command for diagnostics and schema evolution.

### ARAM Mayhem priority

Treat ARAM Mayhem as a first-class mode, not a frontend special case. Preserve raw augment IDs even if catalog resolution fails. Resolve `kiwi.bin.json` records by all supported identifier fields and attempt both Kiwi and Cherry icon roots. Store unresolved IDs for retry after catalog refresh.

## API and frontend migration

1. Introduce agent DTOs matching the current history/profile/rune types.
2. Add a Tauri agent client with connection state and typed errors.
3. Keep existing `api_*` commands as compatibility facades during the transition.
4. Replace direct service calls in commands with agent requests.
5. Subscribe the frontend to incremental agent events.
6. Remove direct LCU access from Tauri only after all callers use the agent.
7. Preserve cached match display and skeleton behavior while agent refreshes.

## Reliability and observability

- Structured logs with component, route, game ID, account PUUID, attempt, and latency.
- Correlation ID for each synchronization cycle.
- Health endpoint exposing LCU state, database migration version, queue depth, last successful sync, and last error.
- Metrics for discovery time, websocket reconnects, summary/detail requests, cache hits, catalog resolution, and projection duration.
- Retry only transient failures; retain explicit errors for authentication, schema, and malformed payloads.
- Add a diagnostic export that excludes credentials and secrets.

## Security and compliance

- Keep LCU credentials in memory only; never write lockfile tokens or Authorization headers to logs.
- Authenticate Tauri-to-agent IPC with a per-session secret.
- Bind local IPC only; do not expose a network listener.
- Treat raw LCU responses and PUUIDs as local private data.
- Do not describe Pengu Loader as Riot-approved without an authoritative Riot statement. Current research found Pengu Loader community documentation and repositories, but no Riot approval evidence. Keep any plugin bridge opt-in, separately packaged, and disabled by default.

## Delivery phases

### Phase 0 — Contract and inventory

- Freeze current frontend DTOs and command behavior.
- Inventory every direct LCU call and every SQLite access.
- Define agent API, events, error model, and migration report.
- Add architecture documentation and decision records.

### Phase 1 — Agent shell

- [x] Add the sibling Rust agent binary target.
- [x] Implement configuration, logging, health, and IPC authentication.
- [x] Implement process supervision from Tauri.
- [x] Start the agent during Tauri setup and keep the UI available through a system tray.
- [ ] Add graceful shutdown and packaged sidecar handling.

### Phase 2 — Database migration

- [x] Add the initial agent schema and migration/import foundation.
- [ ] Add backup creation and a user-visible import report.
- [ ] Verify imported counts/content.
- [ ] Move rune repository writes behind the agent API.

### Phase 3 — LCU supervisor

- [x] Add agent-side LCU discovery and basic recent-history reconciliation.
- [x] Add an agent-side websocket listener with reconnect behavior.
- [ ] Move all credential discovery and REST ownership out of Tauri.
- [ ] Add durable event-to-job scheduling and complete reconciliation.
- [ ] Add state and incremental event APIs.

### Phase 4 — Game collector

- Move match summary/detail collection, raw response logging, detail caching, catalog loading, asset resolution, and ARAM augment handling.
- Add durable job tables and bounded concurrency.
- Add incremental game/projection events.

### Phase 5 — Statistics

- Normalize participant/game data.
- Add most-played, win-rate, KDA, damage/minute, gold/minute, item, rune, and augment projections.
- Add query pagination and indexes.

### Phase 6 — Tauri/frontend cutover

- Route compatibility commands through the agent.
- Replace startup websocket logic with agent lifecycle monitoring.
- Remove duplicate Tauri-side caches and direct LCU paths.
- Validate offline/League-closed, reconnect, migration, and post-game flows.

### Phase 7 — Optional Pengu Loader bridge

- Only after the core agent is stable, evaluate a separately packaged plugin/bridge.
- Limit the bridge to explicit, documented data exchange.
- Do not depend on client injection for collection; LCU REST/websocket remains the source of truth.

## Acceptance criteria

- Closing League does not crash or busy-loop the agent; reopening it resumes synchronization.
- Tauri remains responsive while history, detail, catalog, or statistics work is in progress.
- Existing rune pages and settings survive migration with a backup and report.
- A completed game is collected without opening the profile tab.
- ARAM Mayhem augment IDs remain available even when names/icons cannot be resolved.
- Repeated reconnects and duplicate events do not duplicate games or participants.
- Cached data is visible immediately and incremental updates arrive as records complete.
- No LCU token, PUUID, or raw private response is emitted in ordinary logs.
- Agent failure is visible in the UI and can be recovered without deleting the database.
