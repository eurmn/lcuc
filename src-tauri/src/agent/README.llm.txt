# Agent Module (`src-tauri/src/agent/mod.rs`)

## Overview
`agent/mod.rs` implements the background Tokio worker engine and statistics aggregation platform for LCU Companion. It owns all operations on local SQLite databases (`agent.db` and `global-aram.db`).

## Primary Responsibilities
1. **Collector Loop (`collector_loop`)**:
   - Discovers LCU credentials and resolves summoner PUUID.
   - Reconciles recent 20 matches (`store_recent_games`).
   - Runs exhaustive match history backfill (`backfill_match_history`) when encountering a new account.
   - Spawns background projection refresh only when newly played/stored games are discovered (`has_new == true`), avoiding unnecessary UI churn.
2. **Global ARAM Crawler (`collect_global_aram`)**:
   - Multi-threaded breadcrumb crawler (`worker_count` configurable, default 3).
   - Crawls ARAM Mayhem (`KIWI`) games across discovered player network (`discovered_accounts`).
   - Deduplicates accounts and matches in SQLite and in-memory lock sets (`in_flight_games`).
   - Circuit breaker handles 429 rate limits and consecutive error backoffs.
3. **Statistics Engine & Projections (`build_statistics_response`)**:
   - Computes Win Rate, WPA (Win Probability Added compared to baseline), Pick Rate, KDA, Damage, Gold for Champions, Items, Augments, and Combos.
   - Compares personal vs global statistics side-by-side (`compare` scope).
   - Precomputes and caches aggregated projections in `statistics_projections` table.
   - Provides an in-memory `STATISTICS_QUERY_CACHE` (30s TTL) for instantaneous sorting, filtering, and paging.

## Key Tables Owned
- `agent.db`: `games`, `game_build_facts`, `statistics_projections`, `sync_state`.
- `global-aram.db`: `aram_games`, `aram_participants`, `discovered_accounts`, `config`, `statistics_projections`.

## Usage & Consumers
- Managed in-process via Tokio background tasks in `src-tauri/src/agent_supervisor.rs` and `src-tauri/src/lib.rs`.
- Commands dispatched via `src-tauri/src/commands/agent.rs`.
