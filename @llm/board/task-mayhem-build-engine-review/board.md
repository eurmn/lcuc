# Task Board: task-mayhem-build-engine-review

## Task Details
- **Task ID**: `task-mayhem-build-engine-review`
- **Task Name**: Review and Diagnostic of Mayhem Build Engine Optimization Implementation
- **Status**: In Progress — Comprehensive Review & Diagnostic
- **Created**: 2026-11-01
- **Sub-Board**: `@llm/board/task-mayhem-build-engine-review/board.md`
- **Reference Walkthrough**: `@llm/docs/historical/walkthrough/mayhem_build_engine_full_optimization_walkthrough.md`
- **Log To**: `@llm/BOARD.md`

## Problem Context
The previous AI attempted to implement a 4-wave optimization plan across `src-tauri/src/build_engine/` and frontend Svelte components without verification. The user reported that there are plenty of errors (noted in rust-analyzer/LSP and codebase) and requested a thorough review to determine what went wrong, what goals were stated, and what broke.

## Investigation Scope
- `src-tauri/src/build_engine/models.rs`
- `src-tauri/src/build_engine/item_catalog.rs`
- `src-tauri/src/build_engine/cache.rs`
- `src-tauri/src/build_engine/bitset_miner.rs`
- `src-tauri/src/build_engine/host.rs`
- Frontend components: `MayhemChampSelectView.svelte`, `MayhemCombosCard.svelte`, `MayhemBuildSimulator.svelte`
- Cross-crate / IPC references across Tauri commands and subprocesses

## Sub-Board Log Entries

### [@orchestrator] 2026-11-01 — Fixes Applied & Verified
- Fixed `item_catalog.rs`: Added `#[serde(default)]` to `item_metadata` so `legendary_items.json` parses successfully and avoids catalog wipeout.
- Fixed `cache.rs`:
  - Added `where K: Clone + Eq + std::hash::Hash` to `evict_if_over_capacity`.
  - Replaced `ok()?` with `if let Ok(mut cache)` in `cache_result`.
- Fixed `host.rs`:
  - Completely scoped `LAST_IPC_SEND.lock()` before and after `tokio::time::sleep(...).await`, eliminating non-`Send` guard across `.await` and restoring `Send` for Tauri command futures.
- Fixed `bitset_miner.rs`:
  - Result cache connected: `compute_build_recommendation` now checks `get_cached_result` and stores results via `cache_result`.
  - Candidate mapping corrected: `ItemRecommendation.win_rate` preserves `conservative_wr` (e.g. ~54%), while `lift` carries `delta_wpa` (+4%), fixing the UI display so it renders expected WR badges instead of fractional 4% WR.
- Restored frontend components:
  - `GameEntityPopover.svelte`: Restored `<img>` preview rendering for non-augment entities (champions and items).
  - `AugmentIcon.svelte`: Removed conflicting iconify classes from text fallback glyphs.
  - `MayhemBuildSimulator.svelte`: Restored clean component, removing erroneous `championId: 0` background queries and unused states.
  - `MayhemChampSelectView.svelte`: Fixed cache dedupe to check `lastRecKey` against current inputs instead of comparing against recommendations (`next_items`).
  - `MayhemCombosCard.svelte`: Restored clean formatting functions (`formatPct`, `formatWpa`) and polling operation tracking.


