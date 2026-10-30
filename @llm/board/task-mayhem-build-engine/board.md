# Task Board: task-mayhem-build-engine

## Task Details
- **Task ID**: `task-mayhem-build-engine`
- **Task Name**: Modularize Mayhem Build Creation Engine Backend into Independent Subprocess
- **Status**: Completed (Backend Modularization & Self-Verification)
- **Created**: 2026-10-29

---

## Context & Initial Request
Offload Mayhem build creation and recommendation queries into a dedicated background subprocess (`lcu-build-engine`). Heavy SQLite scans across participant facts and 64-bit vertical bitset intersection mining risk inducing stutter or latency in the main Tauri UI process during Champion Select or live in-game item shopping.

Decouple computation into an independent child process with line-delimited JSON IPC over standard I/O pipes, protected by an in-process fallback mechanism and an in-memory TTL cache.

---

## Architecture Decisions

1. **Subprocess Binary Target (`lcu-build-engine`)**:
   - Registered `[[bin]] name = "lcu-build-engine" path = "src/bin/lcu_build_engine.rs"` in `src-tauri/Cargo.toml`.
   - Dedicated entry point executing `lcu_companion_lib::build_engine::run_build_engine_process()`.

2. **JSON IPC Protocol over `stdin`/`stdout`**:
   - Newline-delimited JSON requests (`BuildEngineRequest`) and responses (`BuildEngineResponse`) with monotonic correlation IDs (`id: u64`).
   - Non-blocking line readers and writers in `src-tauri/src/build_engine/ipc.rs`.

3. **Host Supervisor (`host.rs`)**:
   - Host supervisor in Tauri parent process spawns child with `CREATE_NO_WINDOW` (`0x08000000`) on Windows.
   - Dedicated thread (`build-engine-stdout-reader`) multiplexes incoming responses back to waiting callers using `tokio::sync::oneshot` channels.
   - 1500ms timeout per request with automatic in-process fallback to direct engine calculation if child process fails or exits.
   - Auto-restart logic on process termination.

4. **Vertical Bitset Mining & Bayesian Lift (`bitset_miner.rs`)**:
   - Participant match transactions organized vertically in 64-bit words (`u64`).
   - Bitwise `AND` and popcounts extract joint occurrences and win rates in zero-allocation loops.
   - Bayesian shrinkage ($m = 1.0$ for combos, $m = 20.0$ for candidates) against champion baseline win rate.
   - Computes empirical lift for item-item and item-augment synergies.
   - Dynamically selects boots (suppressed if already owned), determines top 2 non-boot recommendations, and completes a greedy 6-item build.

5. **In-Memory Cache (`cache.rs`)**:
   - Thread-safe static `RwLock<HashMap<u32, CachedIndexEntry>>` with 5-minute TTL.
   - Cache hits skip disk queries, enabling sub-millisecond query responses (<1ms).

6. **SQLite Data Loading (`data_loader.rs`)**:
   - Loads participant fact records from `global-aram.db` (`aram_participants`), falling back to `agent.db` (`game_build_facts`).
   - Read-only connections with 3-second busy timeout.
   - Automatic exclusion of consumables, trinkets, and Poro-Snax (`2050`, `2052`).

---

## File Ownership & Modified Files

### Build Engine Subprocess & Logic
- `src-tauri/Cargo.toml`: Added binary target `lcu-build-engine`.
- `src-tauri/src/bin/lcu_build_engine.rs`: Subprocess binary entry point.
- `src-tauri/src/build_engine/mod.rs`: Module exports and child entry wrapper.
- `src-tauri/src/build_engine/models.rs`: Strongly typed IPC request/response DTOs and build data structures.
- `src-tauri/src/build_engine/data_loader.rs`: Read-only SQLite fact loader.
- `src-tauri/src/build_engine/bitset_miner.rs`: Vertical bitset indexing, combo evaluation, scoring, and unit tests.
- `src-tauri/src/build_engine/cache.rs`: 5-minute TTL memory cache for `ChampionTransactionIndex`.
- `src-tauri/src/build_engine/ipc.rs`: Child stdio JSON line loop.
- `src-tauri/src/build_engine/host.rs`: Tauri supervisor, process launcher, stdout reader, and fallback handler.

### Tauri Command & Lifecycle Integration
- `src-tauri/src/commands/build_engine.rs`: Registered `api_compute_mayhem_build_recommendation` command.
- `src-tauri/src/commands/mod.rs`: Re-exported build engine command module.
- `src-tauri/src/lib.rs`: Handler registration and app setup hook `ensure_build_engine_process_started()`.

---

## Related Walkthroughs
- `@llm/docs/historical/walkthrough/build_engine_modularization_walkthrough.md`
- `@llm/docs/historical/walkthrough/detached_native_rust_overlay_walkthrough.md`

---

## Sub-Board Log Entries

### [@worker-engine] 2026-10-29 18:30
- Initialized `src-tauri/src/build_engine/` subsystem and data models.
- Implemented read-only SQLite facts loader for `global-aram.db` and `agent.db`.
- Created vertical bitset indexer and Bayesian shrinkage candidate ranking engine.
- Added in-memory caching layer with 5-minute TTL.
- Implemented stdio JSON line IPC protocol for child process (`lcu-build-engine`).
- Implemented `BuildEngineSupervisor` in Tauri host with Windows `CREATE_NO_WINDOW` execution, multiplexed oneshot dispatch thread, 1.5s timeout, and fallback.
- Bound Tauri command `api_compute_mayhem_build_recommendation` in `lib.rs`.

### [@worker-verify] 2026-10-29 18:45
- **Unit Test**: Ran `cargo test --manifest-path src-tauri/Cargo.toml --lib build_engine::bitset_miner::tests`.
  - Output: `test build_engine::bitset_miner::tests::test_bitset_intersection_and_recommendation ... ok (1 passed)`.
- **Compiler Check**: Ran `cargo check --manifest-path src-tauri/Cargo.toml --bins`.
  - Output: `Finished dev profile [unoptimized + debuginfo] target(s)` clean across all binary targets (`lcu-companion`, `lcu-overlay`, `lcu-companion-agent`, `lcu-build-engine`).

### [@worker-next-steps] Next Steps for UI / Overlay Integration
1. **Native Overlay Hookup**: Connect `lcu-overlay` or live game state machine to query `api_compute_mayhem_build_recommendation` (or IPC bridge) when owned items/augments change.
2. **Frontend Svelte Presentation**: Expose recommendations through Tauri command wrapper in `src/lib/api/` for Champ Select and live in-game HUD popovers.
3. **Index Prewarming**: Optionally precompute champion transaction index during Champion Select lock-in to guarantee 0ms latency when entering match.
