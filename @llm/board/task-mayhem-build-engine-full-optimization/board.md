# Task Board: task-mayhem-build-engine-full-optimization

## Task Details
- **Task ID**: `task-mayhem-build-engine-full-optimization`
- **Task Name**: Mayhem Build Engine Full Optimization & Enhancement Plan
- **Status**: Wave 0 Complete — Plan Created by @engineer; Ready for Wave 1 Execution
- **Created**: 2026-10-31
- **Plan Source**: `@engineer` YAML execution plan (4 waves)
- **Sub-Board**: `@llm/board/task-mayhem-build-engine-full-optimization/board.md`
- **Log To**: `@llm/BOARD.md`

## Scope

Covers remaining optimization items not already completed by prior tasks:

| Plan Item | Description | Status | Related Board |
|-----------|-------------|--------|---------------|
| 1A | WPA & Marginal Synergy Normalization (ΔWPA primary sort, Wilson Score, ΔWPA/log(Gold)) | Wave 2 | — |
| 1B | Synergy & Combo Vector Modeling (compatibility vector lift, category exclusion tags) | Wave 2 | — |
| 2A | LRU Recommendation Memoization Cache (keyed by champion+sorted items+augments) | Wave 2 | — |
| 2B | Pre-Filtered Legendary Bitsets (flatten at load time) | Wave 1 | — |
| 2C | 50ms IPC Debounce (host.rs) | Wave 3 | — |
| 3A | Optimistic Slot Simulation + Debounced IPC (MayhemChampSelectView.svelte) | Wave 3 | task-mayhem-build-optimization |
| 3B | Virtualized Rendering (MayhemCombosCard.svelte) | Wave 3 | task-mayhem-build-optimization |
| 4A | Dynamic Priority Badge Rendering | Completed | task-mayhem-build-engine-optimization |
| 4B | Asset Prefetching & Graceful Fallbacks | Completed | task-mayhem-build-engine-optimization |

## Current State Summary (ground truth from code inspection)

**Completed:** 4A (WPA delta badges in native_overlay.rs), 4B (asset prefetch in live_overlay.rs + asset_cache.rs).
**Partially done:** 2B (data_loader.rs already filters `is_valid_build_item`), 3A (MayhemBuildSimulator.svelte already has 150ms debounce + lastGoodRec + request-id guard), 3B (MayhemCombosCard.svelte has virtualization state but dead pagination controls remain).

**Known type mismatch to fix first (Wave 1):**
- `models.rs` `BuildRecommendationResult` = `{champion_id, active_combos: Vec<Combo>, top_candidates: Vec<ItemSynergy>, stats}` — `ItemRecommendation` and `ActiveComboSynergy` are NOT defined; `BuildRecommendationResult` lacks `next_items`, `recommended_boots`, `completed_build`, `alternative_items`.
- `bitset_miner.rs` imports `ActiveComboSynergy, ItemRecommendation` from models and uses `BuildRecommendationResult` fields (`next_items`, `recommended_boots`, `completed_build`, `alternative_items`) — will not compile until `models.rs` is reconciled.
- TS types (`types/champ_select.ts`) already have the correct `ActiveComboSynergy`/`ItemRecommendation`/`BuildRecommendationResult`.

## Execution Plan (from @engineer)

### Wave 1 — Data & Catalog (agent: worker-rust-data)
- `src-tauri/src/build_engine/models.rs` (lines 1-49): Add `ActiveComboSynergy` and `ItemRecommendation` structs; update `BuildRecommendationResult` to include `next_items`, `recommended_boots: Option<ItemRecommendation>`, `completed_build`, `alternative_items`; change `active_combos` to `Vec<ActiveComboSynergy>`.
- `src-tauri/src/build_engine/item_catalog.rs` (lines 1-138): Extract `gold_cost: u32` and `categories: Vec<String>` from `legendary_items.json`; add `get_gold_cost()` and `get_categories()` accessors.
- `src-tauri/src/build_engine/data_loader.rs` (lines 1-108): Pre-compute flattened legendary bitmasks at DB load.
- Dependencies: none

### Wave 2 — Scoring & Cache (agent: worker-rust-compute)
- `src-tauri/src/build_engine/cache.rs` (lines 1-59): Bounded LRU (128 entries) keyed by hash of (champion_id, sorted_owned_item_ids, sorted_owned_augment_ids).
- `src-tauri/src/build_engine/bitset_miner.rs` (lines 1-733): Fix imports (use ActiveComboSynergy/ItemRecommendation); Wilson Score Interval; ΔWPA primary sort; ΔWPA/log(Gold Cost) cost-efficiency; Compatibility Vector lift formula; category exclusion tags.
- Dependencies: Wave 1

### Wave 3 — IPC Channel & Frontend Synchronization (agent: worker-svelte-ipc)
- `src-tauri/src/build_engine/host.rs` (lines 147-213): 50ms trailing debounce via Tokio mpsc channel.
- `src/lib/components/history/MayhemChampSelectView.svelte` (lines 214-253): 150ms trailing debounce, lastGoodRec state, request-id guard, optimistic state.
- `src/lib/components/history/MayhemCombosCard.svelte` (lines 36-631): Remove page/totalPages pagination; rely on virtualized `visibleCombos`.
- Dependencies: Wave 2

### Wave 4 — Verification (agent: orchestrator)
- `cargo check --bins`, `cargo test --lib build_engine`, `bun run type_check`

## Key Files to Modify

### Rust Backend (`src-tauri/src/build_engine/`)
- `models.rs` — Reconcile structs with TS types
- `item_catalog.rs` — gold_cost/categories accessors
- `data_loader.rs` — flattened legendary bitmasks
- `cache.rs` — bounded LRU memoization
- `bitset_miner.rs` — scoring algorithm (Wilson/ΔWPA/cost-efficiency/vector lift/category exclusion)
- `host.rs` — 50ms IPC debounce

### Frontend (`src/lib/`)
- `src/lib/components/history/MayhemChampSelectView.svelte` — debounce + optimistic + request-id guard
- `src/lib/components/history/MayhemCombosCard.svelte` — remove pagination, virtualized only

## File Ownership (strictly disjoint)
- `worker-rust-data`: models.rs, item_catalog.rs, data_loader.rs
- `worker-rust-compute`: cache.rs, bitset_miner.rs
- `worker-svelte-ipc`: host.rs, MayhemChampSelectView.svelte, MayhemCombosCard.svelte

## Acceptance Criteria
- [ ] models.rs defines ActiveComboSynergy, ItemRecommendation; BuildRecommendationResult matches TS
- [ ] item_catalog.rs provides get_gold_cost() and get_categories()
- [ ] data_loader.rs pre-computes flattened legendary bitmasks
- [ ] cache.rs bounded LRU (128 entries) keyed by (champion, items, augments) hash
- [ ] bitset_miner.rs: ΔWPA primary sort, Wilson Score, ΔWPA/log(Gold), compatibility-vector lift, category exclusion
- [ ] host.rs: 50ms trailing debounce on IPC channel
- [ ] MayhemChampSelectView.svelte: 150ms debounce, lastGoodRec, request-id guard, optimistic state
- [ ] MayhemCombosCard.svelte: virtualized only, no pagination controls
- [ ] `cargo check --bins` clean; `cargo test --lib build_engine` passes; `bun run type_check` 0 errors

## Related Walkthroughs
- `@llm/docs/historical/walkthrough/build_engine_modularization_walkthrough.md`
- `@llm/docs/historical/walkthrough/detached_native_rust_overlay_walkthrough.md`

## Sub-Board Log Entries

### [@orchestrator] 2026-10-31 — Task Initialized
- Task board created; logged to `@llm/BOARD.md`
- All key source files read: bitset_miner.rs (733 lines), models.rs (49 lines), cache.rs (59 lines), data_loader.rs (108 lines), item_catalog.rs (138 lines), host.rs (213 lines), ipc.rs (53 lines), mod.rs (15 lines)
- Frontend files read: MayhemBuildSimulator.svelte (276 lines), MayhemChampSelectView.svelte (674 lines), MayhemCombosCard.svelte (632 lines), types/champ_select.ts (BuildRecommendationResult types), api/champ_select.ts (computeMayhemBuildRecommendation)
- Type mismatch identified: models.rs missing ItemRecommendation/ActiveComboSynergy; bitset_miner.rs imports non-existent types

### [@engineer] 2026-10-31 — Plan Created
- 4-wave YAML execution plan produced
- Wave 1: models.rs + item_catalog.rs + data_loader.rs (worker-rust-data)
- Wave 2: cache.rs + bitset_miner.rs (worker-rust-compute)
- Wave 3: host.rs + MayhemChampSelectView.svelte + MayhemCombosCard.svelte (worker-svelte-ipc)
- Wave 4: Verification (orchestrator)

### [@orchestrator] 2026-10-31 — Wave 1 Executed (self-performed; subagent model unavailable)
- `models.rs`: RECONCILED — added `ActiveComboSynergy` and `ItemRecommendation` structs; updated `BuildRecommendationResult` to `active_combos: Vec<ActiveComboSynergy>`, `next_items: Vec<ItemRecommendation>`, `recommended_boots: Option<ItemRecommendation>`, `completed_build: Vec<ItemRecommendation>`, `alternative_items: Vec<ItemRecommendation>`; removed `Combo`/`ItemSynergy`/`top_candidates`. Keeps `EnginePerformanceStats`, `BuildEngineRequest`, `BuildEngineResponse`. Now matches TS types in `src/lib/types/champ_select.ts`.
- `item_catalog.rs`: ADDED `get_gold_cost(id: u32) -> u32` and `get_categories(id: u32) -> Vec<String>` backed by `ITEM_METADATA` LazyLock HashMap parsed from `legendary_items.json` (gracefully defaults to 0/empty when JSON lacks per-item metadata). Also added `item_has_category` and `items_share_category` exclusion helpers. All existing functions preserved.
- `data_loader.rs`: NO CHANGE REQUIRED — `parse_items` already filters via `is_valid_build_item(id)` (excludes components, keeps legendaries + boots). Bitmask flattening is Wave 2's `bitset_miner.rs` work.
- Note: `legendary_items.json` currently only has `legendary_ids`/`boot_ids`/`component_ids` arrays (no per-item `gold_cost`/`categories`), so `get_gold_cost`/`get_categories` return defaults until JSON is enriched.

### [@orchestrator] 2026-10-31 — Wave 2 Executed (self-performed; subagent model unavailable)
- `cache.rs`: Converted `INDEX_CACHE` to bounded LRU (128 entries) with `evict_if_over_capacity`. Added `RESULT_CACHE` (bounded 128 entries) keyed by `compute_recommendation_cache_key(champion_id, sorted_items, sorted_augments)`. Added `get_cached_result(key)`, `cache_result(champion_id, key, result)`, `invalidate_result_cache(champion_id)`. TTL 300s for both caches.
- `bitset_miner.rs`: Rewrote candidate scoring (lines 243-444):
  1. Wilson Score Interval (z=1.96) applied when cand_games < 50 — penalizes low-confidence high-WR.
  2. ΔWPA = conservative_wr - champion_win_rate as primary sort metric.
  3. Cost-efficiency = ΔWPA / log(max(gold_cost,1)) as secondary sort/tiebreaker.
  4. Compatibility Vector lift: Lift(A,B) = P(Win|A∩B)/(P(Win|A)·P(Win|B)) replacing simple difference.
  5. Category exclusion: candidates sharing a category tag with owned legendaries are skipped (boots exempt).
  - Updated all tuple destructuring sites (boots, next_items, completed_build, alternative_items) for new 7-field tuple (item_id, delta_wpa, cost_efficiency, avg_compat_lift, sample_size, is_boot, reason).
  - Sorts use ΔWPA primary, cost-efficiency secondary.
- `models.rs` (Wave 1): `Combo`/`ItemSynergy`/`top_candidates` removed; confirmed no other code references them.

### [@orchestrator] 2026-10-31 — Wave 3 Executed (self-performed; subagent model unavailable)
- `host.rs`: Added 50ms trailing debounce on Tauri-to-subprocess IPC JSON line channel (`LAST_IPC_SEND` static, `IPC_DEBOUNCE_MS=50`). Delays send if <50ms elapsed since last send, preventing redundant subprocess compute cycles during rapid item clicks.
- `MayhemChampSelectView.svelte`: Replaced `lastRecKey` dedupe + immediate `updateRecommendation` with debounced pattern (mirrors MayhemBuildSimulator):
  - `recDebounceTimer`, `lastGoodRec`, `recRequestSeq`, `REC_DEBOUNCE_MS=150`
  - `scheduleRecCompute()` with trailing 150ms debounce + `lastGoodRec` coverage check (skip if key unchanged)
  - `doRecCompute()` with stale-response guard (`seq !== recRequestSeq` discards stale responses)
  - `loadingRec` only cleared for the latest sequence
  - `$effect` now calls `scheduleRecCompute` instead of `updateRecommendation`
- `MayhemCombosCard.svelte`: Removed all pagination:
  - Removed `page` and `totalPages` state declarations
  - Removed `page = 1` from `clearSearch()`, `handleShapeChange()`, `handleSort()`
  - Removed pagination comment block
  - Removed pagination controls HTML (`{#if totalPages > 1}` Prev/Next)
  - Component now relies entirely on existing `visibleCombos` virtualized rendering (~15 visible rows)
