# Task Board: task-mayhem-synergy-build-engine

- **Task ID**: `task-mayhem-synergy-build-engine`
- **Task Name**: Implement ARAM Mayhem Hierarchical Bayes Build Engine, Packed u64 Combo Miner & Live 3-Card Augment Advisor
- **Status**: Completed
- **Created**: 2026-11-02
- **Completed**: 2026-11-02

## Overview
Implement the next-generation ARAM Mayhem build intelligence and synergy engine:
1. Packed `u64` single-pass n-gram miner and 4-level Hierarchical Empirical Bayes engine with Wilson LCB.
2. Pairwise & 3-way multi-resource synergy lift ($\Delta\text{syn}$) with kit-eligibility gating.
3. Logit-damped build simulator with active synergy saturation and 3-choice augment draft & non-repeating reroll EV advisor.
4. Clean IPC commands (`compute_mayhem_build_recommendation`, `query_mayhem_combos`) with live client game auto-sync.
5. High-performance Svelte 5 frontend with 60 FPS virtualization, build power bar, and 3-card augment draft panel.

## Stage Tracking
- [x] Stage 1: Fast Codebase Discovery & Contract Freeze
- [x] Stage 2: Rust Backend — Packed `u64` Miner & 4-Level Bayesian Engine
- [x] Stage 3: Tauri Commands & Live Client Data API
- [x] Stage 4: Svelte 5 Frontend Integration
- [x] Stage 5: Verification & Walkthrough Archival

## Stage Execution Log
### Stage 1: Recon & Contract Freeze
- Inspected historical walkthroughs and `@llm/board/task-mayhem-build-optimization/board.md`.
- Mapped existing build engine (`src-tauri/src/build_engine/`), agent statistics (`src-tauri/src/agent/mod.rs`), and live client poller (`src-tauri/src/services/live_game.rs`).
- Froze the Rust <-> TypeScript DTO contract with complete dual-casing support (camelCase + snake_case mirror fields) to guarantee 100% interoperability with existing components while meeting strict Tauri v2 IPC standards.
- DTO contracts locked:
  - `MayhemBuildRecommendationRequest` / `MayhemBuildRecommendationResponse`
  - `ActivePairSynergyDto` (with pure synergy lift $\Delta\text{syn}$ & LCB)
  - `AugmentCardEvalDto` (with score, kit lift, reroll EV, upgrade prob, lookahead combo callouts)
  - `MayhemComboDto` (with confidence grades, WPA, and $\Delta\text{syn}$)
  - `LiveGameState` (with 800ms timeout and augment selection checkpoint indicators)

### Stage 2: Rust Backend & Packed `u64` Miner
- Implemented `src-tauri/src/agent/mayhem_engine.rs`:
  - `pack_combo_key` / `unpack_combo_key` encoding 64-bit keys for kinds 0..=10 with canonical sorting.
  - Single-pass n-gram builder reading participant records from `global-aram.db` with fallback to `agent.db`.
  - Non-build item filtering (`2050`, `2052`, `220013`).
  - 4-level Hierarchical Empirical Bayes ($m_c=30, m_1=25, m_{\text{pers}}=10, m_{\text{glob}}=50$) and Wilson LCB ($z=1.28155$).
  - Pairwise and 3-way multi-resource synergy lift ($\Delta_{\text{syn}}$) with kit-eligibility gating ($N_{xc} > 0 \land N_{yc} > 0$).
  - Log-odds damped build simulator with active synergy saturation ($\tanh$) and geometric decay ($\gamma=0.70$).
  - 3-card candidate scorer with lookahead combo partner detection, standard error penalty, and confidence letter grades (S+..D).
  - Non-repeating reroll EV and upgrade probability calculator over unseen pool $U_{T, c}$.
  - Cache layer using thread-safe `Arc<RwLock<Option<MayhemSynergyIndex>>>`.
  - Hooked cache invalidation into `src-tauri/src/agent/mod.rs`.

### Stage 3: Tauri IPC & Live Game API
- Added `compute_mayhem_build_recommendation` and `query_mayhem_combos` commands in `src-tauri/src/commands/build_engine.rs`.
- Registered commands in `src-tauri/src/lib.rs`.
- Enhanced `src-tauri/src/services/live_game.rs` with 800ms timeout and augment checkpoint detection (levels 3, 7, 11, 15).
- Created TypeScript IPC wrappers in `src/lib/api/champ_select.ts` and exported in `src/lib/api/agent.ts`.

### Stage 4: Svelte 5 Frontend Integration
- Created `src/lib/components/history/MayhemAugmentAdvisorCard.svelte`:
  - Live 3-card augment evaluation with BEST PICK and REROLL badges, upgrade probability, and lookahead partner callouts.
  - Checkpoint banner, Command Line Reroll toggle (+1 tier simulated upgrade), and manual slot selection tabs.
- Rewrote `src/lib/components/history/MayhemCombosCard.svelte`:
  - 60 FPS fixed-row-height virtualization (~15 DOM nodes rendered for hundreds of combos).
  - Category filtering pills ("All Combos", "Item + Augment", "Augment + Augment", "Item + Item", "Augment + Champ", "Item + Champ", "3-Way Champion").
  - Micro-badges for Combo +WPA%, Pure Synergy Lift $\Delta_{\text{syn}}$, and Confidence Grade.
- Preserved and verified `MayhemBuildSimulator.svelte` optimistic pending state and subtle update pulse.
- Updated `MayhemChampSelectView.svelte`:
  - Integrated `MayhemAugmentAdvisorCard`.
  - Added Log-Odds Build Power Bar displaying baseline win rate, predicted win rate (+WPA%), and active synergy pills.
  - Connected 1-click live client game inventory sync into simulator slots.
  - Maintained 150ms IPC debouncing and `lastGoodRec` flicker-free protection.

### Stage 5: Verification, Skeptical Review & Archival
- Verified contract parity across Rust DTOs and TypeScript interfaces in `src/lib/types/champ_select.ts`.
- Skeptical Review & Critical Fixes:
  1. **Fixed `query_mayhem_combos` Category Filtering**: `ThreeWayChamp` filter was skipping all rows due to mismatched category names; now properly categorizes and returns 3-way champion combos (`c_id > 0` with 2 entities).
  2. **Fixed Synergy Lift in `query_mayhem_combos`**: Replaced un-shrunk raw win rate subtraction with true 4-level Hierarchical Empirical Bayes shrinkage, Kit Lift ($\Delta_{\text{champ}}$), Pure Synergy Lift ($\Delta_{\text{syn}}$), and Wilson LCB ($z=1.28155$).
  3. **Included Champion Component for All Champion Combos**: Champion component is now consistently attached to `components` in `query_mayhem_combos`, enabling accurate frontend categorization and badge rendering.
  4. **Strict `tokio::task::spawn_blocking` Isolation**: Wrapped `compute_mayhem_recommendation` and `engine_query_combos` inside `spawn_blocking` to avoid blocking Tokio worker threads while holding read locks.
  5. **Resolved 3x Unseen Reroll Pool Duplication**: Deduplicated 3-alias entries in `AUGMENT_METADATA` and added `locked_augments` to `seen_set` so player-owned augments never reappear in the draft pool.
  6. **Empty Pool ($U_{T, c} = \emptyset$) Boundary Guard**: When the unseen pool is empty, `should_reroll` defaults to `false` and cards with negative scores are not erroneously flagged as "REROLL".
  7. **Enhanced Live Game Checkpoint Range**: Levels 1..=3 are now detected as Slot 1 draft checkpoint.
  8. **Rust Compilation Fixes**:
     - Derived `#[derive(Debug, Clone, Copy, PartialEq)]` on `EvaluatedResource` in `src-tauri/src/agent/mayhem_engine.rs` so `locked_aug_resources.clone()` and `locked_item_resources.clone()` compile cleanly.
     - Removed unused `PathBuf` import from `src-tauri/src/agent/mayhem_engine.rs`.
     - Validated all struct derives and imports in `mayhem_engine.rs` and `src-tauri/src/commands/build_engine.rs`.
- Documented complete architecture and implementation details in `@llm/docs/historical/walkthrough/mayhem_synergy_build_engine_walkthrough.md`.

### [@worker-rust] 2026-11-02 16:00
- Resolved `EvaluatedResource: Clone` compilation failure and removed unused `PathBuf` import.
- Verified all struct derives across `mayhem_engine.rs` and `build_engine.rs`.

### [@reviewer-rust] 2026-11-02 16:05
- Derived `Default` on `EvaluatedResource` alongside `Debug, Clone, Copy, PartialEq`.
- Added `#![allow(non_snake_case)]` to both `src-tauri/src/agent/mayhem_engine.rs` and `src-tauri/src/commands/build_engine.rs` to suppress rustc warnings regarding dual-cased IPC DTO field and parameter mirrors.
- Optimized `all_locked` combination in `compute_mayhem_recommendation`: pre-allocated capacity `Vec::with_capacity(...)` and extended using zero-allocation `.iter().copied()` instead of cloning whole intermediate vectors.


