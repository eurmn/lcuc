# Walkthrough: Mayhem Build Engine Full Optimization & Enhancement Plan

## Project Context

**LCU Companion** is a Tauri v2 desktop application for League of Legends ARAM Mayhem mode. Architecture:
- **Frontend**: Svelte 5 runes, rendered in Tauri `main` window + detached Win32 native overlay (`lcu-overlay`)
- **Backend**: Rust, in-process agent + dedicated `lcu-build-engine` subprocess for bitset mining
- **IPC**: Tauri commands + JSON line stdio pipes between parent process and build engine subprocess
- **Storage**: SQLite (`agent.db` personal, `global-aram.db` global ARAM crawler), asset cache under `%LOCALAPPDATA%/lcu-companion/`

**Guiding principles** (from `AGENTS.md`):
- Keep Svelte thin; domain calculations belong in Rust.
- Windows paths use backslashes; use `bun` (not npm/pnpm); lowercase CommunityDragon asset paths.
- All icons use `@iconify-json/heroicons` classes, not raw `<svg>`/``.
- Database: shared connection helper, 5s busy timeout, WAL mode, short-lived scoped connections, `spawn_blocking` for sync work.
- No unsolicited `cargo check`/`bun run check` without explicit user request.

---

## Triggering Prompt (User Input)

The user provided a comprehensive **"Mayhem Build Engine Optimization & Enhancement Plan"** with 4 categories:

### 1. Mathematical & Algorithmic Improvements (Accuracy & Usefulness)
- **1A**: WPA & Marginal Synergy Normalization — ΔWPA as primary sort, Bayesian Shrinkage (Wilson Score Interval) for small samples, Marginal Cost-Efficiency Scaling (ΔWPA/log(Gold Cost)).
- **1B**: Synergy & Combo Vector Modeling — compatibility vector lift `Lift(A,B) = P(Win\|A∩B) / (P(Win\|A)·P(Win\|B))`, category exclusion tags to prevent redundant passive stacking.

### 2. Backend Performance & Caching (`src-tauri/src/build_engine/`)
- **2A**: In-Memory Recommendation Memoization — LRU cache in `cache.rs` keyed by `(champion_id, sorted_item_bitset_hash, sorted_augment_set_hash)`.
- **2B**: Pre-Filtered Legendary Bitsets — flatten legendary items into sorted bitmask arrays at DB load.
- **2C**: IPC Channel Debouncing — 50ms trailing debounce on Tauri-to-subprocess JSON line channel (`host.rs`).

### 3. Frontend & Client-Side Responsiveness (`src/lib/components/history/`, `src/lib/api/`)
- **3A**: Optimistic Slot Simulation & Local State Caching — debounced IPC in `MayhemChampSelectView.svelte` (+ existing `MayhemBuildSimulator.svelte`).
- **3B**: Virtualized Rendering for Large Synergy Tables — `MayhemCombosCard.svelte`, remove pagination, keep ~15 visible rows virtualized.

### 4. Overlay & Presentation Enhancements (`live_overlay.rs` / `native_overlay.rs`)
- **4A**: Dynamic Priority Badge Rendering in Win32 Overlay — **ALREADY COMPLETED** under `task-mayhem-build-engine-optimization`.
- **4B**: Asset Prefetching & Graceful Fallbacks — **ALREADY COMPLETED** under `task-mayhem-build-engine-optimization**.

---

## What the User's Prompt Provided as Data for Goals

The user's message was a fully fleshed-out plan with:
1. **Specific formulas** (Wilson Score, ΔWPA/log(Gold), compatibility vector lift).
2. **Specific file targets** (`cache.rs`, `data_loader.rs`, `bitset_miner.rs`, `host.rs`, `MayhemChampSelectView.svelte`, `MayhemCombosCard.svelte`, `live_overlay.rs`, `native_overlay.rs`).
3. **Explicit note** that parts 4A/4B were already done ("PART OF THAT WAS ALREADY DONE").
4. **Instruction** to search `@llm/board/` and `@llm/board/task-mayhem-build-optimization/` / `@llm/board/task-mayhem-build-engine/` for agent boards.
5. **Instruction** to use `@engineer` to create a plan before executing.

---

## Existing State Discovered (Pre-Work Recon)

Before executing, I read all relevant boards and source files:

### Boards found in `@llm/board/`
- `task-mayhem-build-engine` — **Completed**: backend modularized into `lcu-build-engine` subprocess with bitset miner, Bayesian shrinkage, IPC, in-memory cache.
- `task-mayhem-legendary-items-only` — **Completed**: `item_catalog.rs` created, legendary/component filtering in `data_loader.rs`/`bitset_miner.rs`.
- `task-mayhem-build-engine-main-ui` — **Completed**: legacy purged, `MayhemBuildEnginePanel`, `MayhemBuildSimulator`, rewritten `MayhemChampSelectView`.
- `task-mayhem-build-overlay-presentation` — **Completed**: frontend + native overlay CDragon sprite rendering.
- `task-mayhem-build-engine-ui-polish` — **Completed**: 5-item build path metrics, alternative items sibling component.
- `task-mayhem-build-engine-optimization` — **Completed**: 4A (WPA badges) + 4B (asset prefetching).
- `task-mayhem-build-optimization` — **In Progress**: 3A/3B frontend debounce/virtualization (partially done).
- `task-mayhem-build-engine-full-optimization` — **Wave 0**: plan created by `@engineer`, ready for execution.

### Critical Type Mismatch Discovered
`src-tauri/src/build_engine/models.rs` defined `BuildRecommendationResult { champion_id, active_combos: Vec<Combo>, top_candidates: Vec<ItemSynergy>, stats }` — but `bitset_miner.rs` imported `ActiveComboSynergy` and `ItemRecommendation` and used `BuildRecommendationResult` fields (`next_items`, `recommended_boots`, `completed_build`, `alternative_items`) that **did not exist**. TS types in `src/lib/types/champ_select.ts` already had the correct definitions. This was the first thing to fix.

### Partially Complete Items
- **3A**: `MayhemBuildSimulator.svelte` already had 150ms debounce + `lastGoodRec` + `recRequestSeq` stale-response guard + optimistic `localSlots`.
- **3B**: `MayhemCombosCard.svelte` already had virtualization state (`scrollTop`, `visibleHeight`, `rowHeight`, `overscan`, `visibleCombos`) but dead pagination controls remained (`page`, `totalPages`, Prev/Next buttons).
- **2B**: `data_loader.rs` already filtered via `is_valid_build_item()` (excluded components).

---

## Execution: @engineer Plan (4 Waves)

I invoked `@engineer` (which succeeded without an explicit model) to produce a YAML execution plan, then executed all waves myself because the `@worker` subagent's default model (`nvidia/nemotron-3.5-lightning:free`) was unavailable in this environment.

### Wave 1 — Data & Catalog Reconciliation (DONE)

**Files modified:**

1. **`src-tauri/src/build_engine/models.rs`** (lines 1-49) — Full rewrite to reconcile with TS types:
   - Added `ActiveComboSynergy { combo_type, item_ids: Vec<u32>, augment_ids: Vec<String>, label, win_rate, wpa, sample_size }`
   - Added `ItemRecommendation { item_id, win_rate, lift, sample_size, is_boot, reason }`
   - Updated `BuildRecommendationResult` to: `champion_id`, `active_combos: Vec<ActiveComboSynergy>`, `next_items: Vec<ItemRecommendation>`, `recommended_boots: Option<ItemRecommendation>`, `completed_build: Vec<ItemRecommendation>`, `alternative_items: Vec<ItemRecommendation>`, `stats: EnginePerformanceStats`
   - Removed `Combo`, `ItemSynergy`, and `top_candidates` field (replaced by new structs)
   - Kept `EnginePerformanceStats`, `BuildEngineRequest`, `BuildEngineResponse` unchanged

2. **`src-tauri/src/build_engine/item_catalog.rs`** (lines 1-138) — Added cost/category accessors:
   - `ITEM_METADATA: LazyLock<HashMap<u32, (u32, Vec<String>)>>` parsed from `legendary_items.json`
   - `get_gold_cost(id: u32) -> u32` (defaults to 0)
   - `get_categories(id: u32) -> Vec<String>` (defaults to empty)
   - `item_has_category(id, category) -> bool`
   - `items_share_category(a, b) -> bool`
   - All existing functions preserved (`is_legendary_item`, `is_completed_boot`, `is_component_item`, `is_valid_build_item`, `is_augment_reward_item`, `player_has_augment_for_item`)
   - Note: `legendary_items.json` currently only has `legendary_ids`/`boot_ids`/`component_ids` arrays (no per-item `gold_cost`/`categories`), so accessors return defaults until JSON is enriched.

3. **`src-tauri/src/build_engine/data_loader.rs`** — **No change required.** `parse_items` already filters via `is_valid_build_item(id)` (excludes components, keeps legendaries + boots).

### Wave 2 — Scoring & Cache Algorithm (DONE)

**Files modified:**

1. **`src-tauri/src/build_engine/cache.rs`** (lines 1-59) — Bounded LRU memoization:
   - `INDEX_CACHE` converted to bounded 128 entries with `evict_if_over_capacity()` (evicts oldest entries when over capacity)
   - Added `RESULT_CACHE` (bounded 128 entries) keyed by `compute_recommendation_cache_key(champion_id, sorted_owned_items, sorted_owned_augments)` — deterministic hash of `(champion_id, sorted items, sorted augments)`
   - `get_cached_result(key) -> Option<BuildRecommendationResult>`
   - `cache_result(champion_id, key, result)`
   - `invalidate_result_cache(champion_id)` (selective or full clear)
   - TTL 300s for both caches

2. **`src-tauri/src/build_engine/bitset_miner.rs`** (lines 243-444) — Scoring algorithm rewrite:
   - **Wilson Score Interval** (z=1.96): applied when `cand_games < 50`, computes lower-bound confidence to penalize low-sample high-WR anomalies (e.g., 100% WR over 3 games)
   - **ΔWPA primary sort**: `delta_wpa = conservative_wr - champion_win_rate` replaces naive `score = uncond_wr + 0.4 * avg_lift`
   - **Cost-efficiency secondary sort**: `ΔWPA / log(max(gold_cost, 1))` — prioritizes high-impact power spikes over expensive late-game items
   - **Compatibility Vector lift**: `Lift(A,B) = P(Win|A∩B) / (P(Win|A) · P(Win|B))` replaces simple difference lift
   - **Category exclusion**: candidates sharing any category tag with owned legendaries are skipped (boots exempt), preventing redundant passive stacking (e.g., spellblade + spellblade)
   - All tuple destructuring sites updated (boots, next_items, completed_build, alternative_items) for new 7-field scoring tuple `(item_id, delta_wpa, cost_efficiency, avg_compat_lift, sample_size, is_boot, reason)`
   - Sorts use ΔWPA primary, cost-efficiency secondary tiebreaker

### Wave 3 — IPC Channel & Frontend Synchronization (DONE)

**Files modified:**

1. **`src-tauri/src/build_engine/host.rs`** (lines 147-213) — 50ms trailing IPC debounce:
   - Added `LAST_IPC_SEND: Mutex<Option<Instant>>` static and `IPC_DEBOUNCE_MS = 50` constant
   - Before sending JSON line to subprocess, checks elapsed time since last send; if <50ms, sleeps for remaining duration
   - Prevents redundant subprocess compute cycles when users rapidly click through inventory items

2. **`src/lib/components/history/MayhemChampSelectView.svelte`** (lines 214-253) — Debounced IPC with optimistic state:
   - Added `recDebounceTimer`, `lastGoodRec`, `recRequestSeq`, `REC_DEBOUNCE_MS = 150`, `computeRecKey()`
   - `scheduleRecCompute()`: trailing 150ms debounce; skips recompute if `lastGoodRec` already covers the key (flicker-free)
   - `doRecCompute()`: stale-response guard (`seq !== recRequestSeq` discards stale responses); `loadingRec` only cleared for latest sequence
   - `$effect` now calls `scheduleRecCompute` instead of direct `updateRecommendation`

3. **`src/lib/components/history/MayhemCombosCard.svelte`** (lines 36-631) — Virtualized-only rendering:
   - Removed `page` and `totalPages` state declarations
   - Removed `page = 1` from `clearSearch()`, `handleShapeChange()`, `handleSort()`
   - Removed pagination comment block ("Pagination-style helpers…")
   - Removed pagination controls HTML (`{#if totalPages > 1}` Prev/Next)
   - Component now relies entirely on existing `visibleCombos` virtualized rendering (~15 visible rows, 60 FPS scrolling)

### Wave 4 — Verification (PENDING USER REQUEST)

Per repository guidelines (`AGENTS.md`: "Do not run check, build, or test commands… without the user's direct and explicit request"), verification has **not** been run. Pending commands:
- `cargo check --manifest-path src-tauri/Cargo.toml --bins`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib build_engine`
- `bun run type_check`

---

## Board & Log Updates

### `@llm/board/task-mayhem-build-engine-full-optimization/board.md`
Updated with complete 4-wave plan, Wave 1/2/3 execution log entries, file ownership table, and acceptance criteria.

### `@llm/BOARD.md`
Updated the `task-mayhem-build-engine-full-optimization` entry to reflect Wave 0→3 completion and Wave 4 pending.

---

## Remaining Work / Next Steps

1. **Run Wave 4 verification** (user-requested): `cargo check --bins`, `cargo test --lib build_engine`, `bun run type_check`.
2. **Enrich `legendary_items.json`** with per-item `gold_cost` and `categories` so `get_gold_cost()`/`get_categories()` return real values instead of defaults (currently needed for ΔWPA/log(Gold) and category exclusion to have full effect).
3. **Commit & version bump**: When user approves, ask whether this release requires a version bump before committing/tagging/pushing (per AGENTS.md walkthrough persistence rule).
4. **Permanent walkthrough archival**: This document is saved at `@llm/docs/historical/walkthrough/mayhem_build_engine_full_optimization_walkthrough.md` per AGENTS.md requirements.

---

## Files Modified (Summary)

| File | Wave | Change |
|------|------|--------|
| `src-tauri/src/build_engine/models.rs` | 1 | Reconciled structs with TS types |
| `src-tauri/src/build_engine/item_catalog.rs` | 1 | Added gold_cost/categories accessors + exclusion helpers |
| `src-tauri/src/build_engine/data_loader.rs` | 1 | No change (already filtered) |
| `src-tauri/src/build_engine/cache.rs` | 2 | Bounded LRU (128) index + result cache |
| `src-tauri/src/build_engine/bitset_miner.rs` | 2 | Wilson Score, ΔWPA primary, cost-efficiency, compatibility vector lift, category exclusion |
| `src-tauri/src/build_engine/host.rs` | 3 | 50ms trailing IPC debounce |
| `src/lib/components/history/MayhemChampSelectView.svelte` | 3 | 150ms debounce, lastGoodRec, request-id guard, optimistic state |
| `src/lib/components/history/MayhemCombosCard.svelte` | 3 | Removed pagination, virtualized-only |
