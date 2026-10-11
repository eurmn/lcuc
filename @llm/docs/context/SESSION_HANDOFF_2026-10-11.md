# Session Handoff — Agent, Collection, and Statistics Stabilization

Date: 2026-10-11  
Repository: `eurmn/lcuc`  
Working directory: `.`

## Purpose

This document records the implementation work completed during the session and
gives the next agent a reliable route through the repository. It intentionally
does not describe pending product or visual design changes.

## Original handoff request

The session ended with this request, which is preserved here verbatim:

> two things. first, i see `[2026-10-11T20:27:24.761Z] [INFO] [agent.collector] Collector account scan status: puuid_present=true, backfill_complete=true`
> `[2026-10-11T20:27:24.814Z] [DEBUG] [agent.collector] Reconciled 20 recent games; takes very long to finish, and whatever runs after it. i will make some ui changes here in photoshop to show you what we gotta do next. i think this session is too long now, please document everything before answering back to me, updating documentation for what we did this session and creating a .md file for the next agent to know what to do, dont talk about the ui changes there tho, just guide him through what we've been doing and how he should navigate the project yk, create agent.md.new also with a new revamped agent.md for the project lcu-companion.`

The operational meaning is: investigate the long delay after the collector
reports that the account scan/backfill is complete and after it reconciles its
20-game recent window. The documentation requested that follows must guide the
next agent through the current agent, database, statistics, logging, and
validation architecture without discussing unrelated upcoming UI work.

## Architecture reached

The project is a Tauri 2 + Svelte 5 desktop application with a separately
running Rust agent:

```text
Svelte -> Tauri commands -> authenticated loopback IPC -> Rust agent
                                                     -> SQLite
                                                     -> LCU REST/WebSocket
```

The agent is intended to become the authoritative owner of collection,
normalization, statistics, and long-lived database state. Older direct Tauri
services remain for compatibility, so the migration is not complete.

## Files to read first

1. `@llm/PLAN.md` — migration goals, ownership boundaries, and remaining work.
2. `@llm/docs/context/CURRENT_STATE.md` — current architecture and runtime flow.
3. `src-tauri/src/agent/mod.rs` — database setup, IPC, reconciliation, global
   collector, statistics, logging, and runtime loops.
4. `src-tauri/src/agent_supervisor.rs` — agent process lifecycle and IPC auth.
5. `src-tauri/src/commands/agent.rs` — Tauri compatibility commands.
6. `src/lib/api/agent.ts` — typed agent-backed frontend contracts.
7. `@llm/docs/db/DATABASE.md` — database locations and ownership rules.

Use `rg` on `src-tauri/src/agent/mod.rs` for:

```text
run_process
handle_request
collector_loop
collect_global_aram
store_recent_games
store_global_detail
build_statistics_response
global_game_limit
backfill
reconcile
```

## Implemented work

### History and profile data path

- Repaired the corrupted match-history Svelte table and changed champion
  display from IDs to cached champion icon/name data.
- Added League-style match details including result, champion, KDA, CS, gold,
  damage, duration/date, items, and augments.
- Removed the history download action.
- Added skeleton/stale-data behavior, progressive loading, cache merging, and
  structured history logs.
- Added profile/build statistics perspectives backed by typed agent requests.
- Moved WR, WPA, pick rate, filtering, sorting, pagination, context rows, and
  Poro-Snax exclusion into Rust-side statistics preparation.
- Added personal/global/both scope handling, global limit configuration, live
  collector state, queue information, and server-enriched metadata/icon fields.

### Logging and diagnostics

- Added shared structured logging across frontend, Tauri, and agent paths.
- Logs use UTC timestamps, severity, source, message, and optional context.
- Added collector/backfill/global-ARAM/statistics timing and progress messages.
- The agent supervisor reports launch, reachability, health, and startup errors.

### Standalone agent and supervisor

- Added the `lcu-companion-agent` binary.
- Added authenticated newline-delimited JSON IPC over loopback TCP.
- Tauri reserves an ephemeral loopback port and generates a per-process token.
- Tauri launches the existing compiled agent binary when available.
- The fallback path invokes Cargo if the binary is missing.
- Health/status requests expose process reachability and database readiness.

Important: `pnpm tauri dev` does not guarantee that an existing standalone
agent executable is rebuilt after Rust changes. Rebuild and restart with:

```powershell
cargo build --manifest-path src-tauri\Cargo.toml --bin lcu-companion-agent
```

### Collection and persistence

- Recent match reconciliation and detail backfill run in the agent.
- Global ARAM collection follows participant PUUIDs breadth-first in a
  separate database.
- Global collection has a configurable limit, collector state, queue count,
  and progress logs.
- Game and participant writes are idempotent around stable keys.
- Poro-Snax IDs `2050` and `2052` are removed during cleanup and filtered
  before new facts are inserted or aggregated.

Database locations:

```text
%LOCALAPPDATA%\lcu-companion\agent.db
%LOCALAPPDATA%\lcu-companion\global-aram.db
```

Do not add another writer for these files. Prefer agent IPC commands.

### Server-owned statistics

- Rust reads personal and global fact tables.
- Rust computes WR, WPA, KDA, damage, gold, duration, pick rate, champion
  context, and partner rows.
- Rust applies search, low-pick-rate filtering, sorting, and pagination.
- Svelte receives render-ready rows and must not reconstruct aggregates.
- Statistics responses are briefly cached by query dimensions; pagination is
  applied to cached ordered rows.
- Statistics preparation runs in `spawn_blocking` because rusqlite is
  synchronous.

The current cache is process-local and short-lived. It is a performance layer,
not durable precomputation. The next improvement is revision-based
invalidation or materialized aggregate projections updated on fact writes.

### Database contention mitigation

- Agent connections use a shared helper with a five-second busy timeout,
  normal synchronous mode, and in-memory temporary storage.
- Global detail writes run in `spawn_blocking`.
- Statistics reads run in `spawn_blocking`.
- Network requests and JSON parsing should remain outside write transactions.

This reduces async-task blockage but does not yet prove every collection phase
is contention-free. Profile before adding more concurrency.

### Logging

Structured logs use:

```text
[UTC timestamp] [LEVEL] [source] message | context
```

Relevant sources include `agent.collector`, `agent.global-aram`,
`agent.statistics`, and `tauri.agent.supervisor`. Never log credentials,
authorization headers, tokens, or sensitive raw payloads.

## Current observed bottleneck

The latest runtime output includes:

```text
[INFO] [agent.collector] Collector account scan status: puuid_present=true, backfill_complete=true
[DEBUG] [agent.collector] Reconciled 20 recent games
```

This means the account identity was found, the current backfill flag was
complete, and the bounded recent-history pass processed 20 games. It does not
mean the account has only 20 total games. The expensive phase after
reconciliation has not been isolated.

Add phase timing around:

- summary fetch and ARAM filtering
- detail queue construction
- detail requests
- fact persistence
- metadata/cache resolution
- projection/statistics refresh
- collector sleep and wakeup

Useful fields are correlation ID, summary/ARAM/detail counts, queue depth,
fetched/skipped/failed counts, database duration, projection duration, and
total phase duration.

## Recommended next sequence

1. Trace `collector_loop` into reconciliation and backfill without editing.
2. Add phase-level timing logs and reproduce with the standalone binary.
3. Identify whether the delay is network, synchronous SQLite, metadata
   resolution, projection work, or repeated reconciliation.
4. Add durable cursors and completed-job checkpoints for account scans and
   detail queues.
5. Keep writes short and serialized; use bounded `spawn_blocking` operations.
6. Replace the temporary statistics cache with revision-aware projections.
7. Add IPC smoke tests for health, sorting, pagination, collector status, and
   restart recovery.
8. Continue the Tauri compatibility cutover only after recovery behavior is
   verified.

## Validation already used

```powershell
cargo fmt --manifest-path src-tauri\Cargo.toml
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
cargo build --manifest-path src-tauri\Cargo.toml --bin lcu-companion-agent
git diff --check
```

The agent was launched on an isolated loopback port and queried through raw
authenticated IPC. Repeated statistics requests returned `cached=true`,
preserved server-side offset pagination, and returned successful rows.

## Constraints

- Use Windows paths with backslashes.
- Use `pnpm`, never `npm`.
- Use lowercase CommunityDragon paths.
- Keep Svelte thin; calculations belong in Rust.
- Preserve backend cached asset/data-URI behavior.
- Do not use destructive Git commands or revert unrelated worktree changes.
- Do not commit credentials, tokens, or raw sensitive responses.
