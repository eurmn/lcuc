# Current Project State: LCU Companion

Date: 2026-10-11
Repository: eurmn/lcuc
Architecture: Tauri 2.0 (Rust) + Svelte 5 (TypeScript) Desktop App + Standalone Agent (`lcu-companion-agent`)

---

## 1. Architecture & Current State

LCU Companion runs as a client-rendered desktop container with a dedicated background data/crawler agent:

```text
Svelte 5 (UI) -> Tauri IPC Facade -> Authenticated Loopback IPC (JSON-lines)
                                         -> Standalone Rust Agent (`lcu-companion-agent.exe`)
                                         -> SQLite (agent.db & global-aram.db)
                                         -> LCU REST & WebSocket (127.0.0.1:<port>)
```

- **Frontend**: Svelte 5 (Runes), Tailwind CSS v4, Flowbite-Svelte patterns, typed IPC wrappers in `src/lib/api/agent.ts`.
- **Backend / Tauri**: Supervised child process management via `src-tauri/src/agent_supervisor.rs`.
- **Standalone Agent**: `src-tauri/src/agent/mod.rs` owns all data collection, SQLite operations, global ARAM crawling, precomputed projections, and statistics aggregation.

---

## 2. Databases Owned by Agent

Located in `%LOCALAPPDATA%\lcu-companion\`:

1. `agent.db`:
   - `accounts`, `recent_games`, `game_build_facts`
   - `statistics_projections` (precomputed aggregate metrics cached by kind, role, rank)
2. `global-aram.db`:
   - `discovered_accounts` (status tracking: `queued`, `fetching`, `complete`, `failed`, indexed)
   - `aram_games`, `aram_build_facts`
   - `statistics_projections` (global precomputed aggregate metrics)
   - Configuration table for target limits and crawler state

---

## 3. Implemented Features & Work Done

### A. Statistics & Compare Mode
- Server-side Rust aggregation of personal vs. global statistics.
- Metrics calculated:
  - Benchmark-relative WPA (Win Probability Added compared to champion baseline winrate).
  - Separate personal WR/WPA vs. global WR/WPA columns.
  - Pick rates calculated with total matches denominator.
  - Formatted numeric KDA ratio (two decimal places).
  - Poro-Snax item IDs 2050 and 2052 explicitly filtered out.
- Augment facts:
  - Normalized ARAM Mayhem augment IDs stored and queried for items/augments compare tabs.

### B. Precomputed Projections & Invalidation
- Table `statistics_projections` stores precomputed rows in SQLite.
- Facts updates invalidate projections and trigger background re-calculation.
- Query cache `STATISTICS_QUERY_CACHE` provides in-memory sub-millisecond responses for repeated queries.

### C. Persistent Global ARAM Crawler
- Persistent node queue: Discovered participant PUUIDs are stored in `discovered_accounts`.
- Resilient crawl loop:
  - On restart, un-crawled accounts are loaded from DB (`status = 'queued'`).
  - Visited accounts (`complete`, `failed`) are loaded into memory to prevent duplicate fetches.
  - Match deduplication: Checks game IDs against `aram_games` with `uncollected_global_games` before sending LCU detail requests.
  - Node errors mark account `failed` rather than halting crawler loop.

### D. UI Progress Banner
- Top horizontal multi-operation progress banner in `src/App.svelte`.
- Shows live crawler progress (e.g., `124/1000 games`), projection builds, and database updates.

---

## 4. Active Endpoints & IPC Commands

### Agent IPC (`api_agent_request` / loopback TCP):
- `status`: Collector state, connection status, game count.
- `global_aram_config`: Get/set global ARAM crawl limits, active status, queue depths.
- `agent_operations`: List active background tasks with percentage/fraction progress for the UI banner.
- `statistics`: Server-side calculated and precomputed statistics for champions, items, augments, combos.
- `build_statistics`: Raw/compare build facts.
- `history_summary`: Reconciled match history.

---

## 5. Development & Verification Rules

- Rebuild standalone agent executable:
  `cargo build --manifest-path src-tauri\Cargo.toml --bin lcu-companion-agent`
- Frontend type check:
  `pnpm svelte-check`
- Do not run Cargo checks or builds without explicit user instructions.
- All domain statistics and aggregation logic belong in Rust; Svelte is a presentation layer.
