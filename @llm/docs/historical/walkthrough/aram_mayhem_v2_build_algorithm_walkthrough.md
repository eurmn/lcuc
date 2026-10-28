# ARAM Mayhem V2 Build Algorithm & Purchase Order Inference Walkthrough

## Summary
Designed and implemented the **V2 Build Creation Algorithm** for ARAM Mayhem:
1. **Purchase Priority & Order Inference**: Leverages early-ended match data where players only finished 1 or 2 legendaries to definitively isolate 1st item rush purchases and 2nd item follow-ups, blended with slot index distribution.
2. **Coherent Synergies (General Builds)**: Eliminates contradictory item mixes by building step-by-step conditional co-occurrence paths ($1st \to 2nd \to 3rd \to 4th \to 5th$) with strict exclusive item group enforcement (Hydra, Lifeline, Last Whisper, Crit Modifiers).
3. **Adaptive Augment Synergies**: Computes empirical item-augment win rate deltas ($WR(item | augment) - WR(champion)$) with Bayesian shrinkage. Generates dedicated branching adaptive builds upon selecting or simulating augments, and annotates synergizing items with `Synergy: +X% WR` tags.
4. **Sample Size Preservation**: Uses the entire 100% champion dataset for baseline general builds and per-augment slices for synergies rather than over-filtering to empty intersections.
5. **V1 / V2 Algorithm Toggle**: Added right before the Encyclopedia button in both `MayhemChampSelectView.svelte` and `BuildStatisticsView.svelte`, persisted in `localStorage` (`mayhem-build-algorithm-version`).

---

## Changes

### 1. Backend Core (`src-tauri/src/services/history.rs`)
- Updated `MAYHEM_CHAMPION_BUILDS_CACHE` key to `(i64, Vec<String>, String)` to partition cache by champion, augments, and algorithm version (`"v1"` vs `"v2"`).
- `parse_facts_for_v2`: Extracts legendary items, slots, boots, starters, spells, and augments per match.
- `infer_v2_rush_items_from_refs`: Computes rush priority score:
  - Weight 4.0 for matches with exactly 1 completed legendary (`solo_counts`, 100% certainty 1st item purchase).
  - Weight 1.5 for slot-0 in 2-legendary matches (`pair_first_counts`).
  - Slot factor `(5.5 - mean_slot).clamp(0.2, 5.0)`.
  - Bayesian win rate shrinkage.
- `assemble_v2_strategy_for_core`: Builds coherent item progression trees:
  - 1st Item: Inferred Rush Item.
  - 2nd Item: Highest conditional co-occurrence and Bayesian WR partner with 1st item.
  - 3rd Item: Highest co-occurrence with 1st and 2nd items.
  - 4th & 5th Items: Continuation capstones with compatible archetype tags and exclusive group verification.
  - Boots & Starter: Highest-frequency choices from facts matching rush item.
  - Item Tags: Items with positive augment synergy receive `Synergy: +X% WR` tags.
  - Alternatives: Synergized items sorted to the top.
- `compute_mayhem_champion_build_v2`:
  - Produces **General Build 1** (Primary Rush item) and **General Build 2** (Alternative playstyle, e.g. Lethality vs Crit or AP vs AD).
  - Produces **Adaptive Build(s)** when augments are selected (both data-driven and warped templates like `Marksmage`, `ADAPt`, `Apex Inventor`, `Big Shot`).
  - Caches champion augment frequencies in `MAYHEM_CHAMPION_AUGMENTS_CACHE`.
- Updated `compute_mayhem_champion_build`:
  - Added `algorithm_version: Option<String>` parameter, defaulting to `"v2"`.
  - Dispatches to V2 when `"v2"`, leaves V1 legacy pipeline intact when `"v1"`.
- Unit Tests: Added `test_v2_infer_rush_items_from_early_matches`.

### 2. Backend Commands (`src-tauri/src/commands/champ_select.rs`)
- `api_mayhem_champion_build`: Added `algorithm_version: Option<String>` parameter.
- `api_mayhem_team_builds`: Added `algorithm_version: Option<String>` parameter.
- `api_get_live_build_advice`: Passes `None` (defaults to V2).

### 3. Frontend API Client (`src/lib/api/champ_select.ts`)
- `getMayhemChampionBuild(championId, augmentIds, algorithmVersion = "v2")`: Passes version parameter to Tauri command.
- `getMayhemTeamBuilds(championIds, algorithmVersion = "v2")`: Passes version parameter to Tauri command.

### 4. Frontend UI (`src/lib/components/history/`)
- `MayhemChampSelectView.svelte`:
  - Added `buildAlgorithmVersion: "v1" | "v2"` state, persisted in `localStorage`.
  - Added sleek segmented `[V1 | V2]` toggle button immediately preceding the Encyclopedia button.
  - Automatically re-fetches champion and team builds on toggle.
- `BuildStatisticsView.svelte`:
  - Added synchronized `[V1 | V2]` toggle button immediately preceding the Encyclopedia button.

---

## Validation Notes
- Incomplete match purchase priority verified with unit test (`test_v2_infer_rush_items_from_early_matches`).
- Exclusive groups respected for all continuation items.
- V1 / V2 toggling partitions cache keys cleanly.
