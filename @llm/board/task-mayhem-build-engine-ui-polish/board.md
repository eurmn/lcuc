# Task: Mayhem Build Engine UI Layout Polish & Alternative Options

**Task ID**: `task-mayhem-build-engine-ui-polish`
**Status**: In Progress
**Date**: 2026-10-30

## Context & User Requirements
1. **5-Item Build Path Layout**:
   - The current 5 items in the middle card look too small / unpleasing.
   - Resize icons to at least the same size as the boots icon (`h-11 w-11` or `h-12 w-12`).
   - Underneath each item in the 5-item build path, display win rate (`XX.X% WR`), games count (`XXg`), and engine reasoning/synergy info.
2. **Alternative Items Component**:
   - Add a new section/component underneath the 3 cards inside the `MayhemBuildEnginePanel` card (as a sibling of the 3-card grid).
   - Displays alternative items not included in the primary 5-item build path.
   - For each alternative item, show its engine decision metrics (WR, Lift, games, and the decision justification from the algorithm, without using the literal word "why").
   - Click to equip/add to build simulator.
3. **Backend & IPC Contract**:
   - `ItemRecommendation` struct already has `item_id`, `win_rate`, `lift`, `sample_size`, `is_boot`, `reason`.
   - Update `BuildRecommendationResult` to return:
     - `completed_build: Vec<ItemRecommendation>` (or provide item details for the 5 completed build items).
     - `alternative_items: Vec<ItemRecommendation>`: candidate items from the ranking that didn't make the top 5 build path or next 2 items.
   - Update frontend types in `src/lib/types/champ_select.ts`.

## Plan
1. Backend (`src-tauri/src/build_engine/models.rs` & `bitset_miner.rs`):
   - In `BuildRecommendationResult`:
     - Change `completed_build: Vec<ItemRecommendation>`
     - Add `alternative_items: Vec<ItemRecommendation>`
   - In `bitset_miner.rs`:
     - Collect full `ItemRecommendation` objects for `completed_build` (including owned items and newly filled items).
     - Collect top alternative non-boot legendaries (e.g. top 6-8) that are not in `completed_build`.
     - Update unit tests.
2. Frontend Types (`src/lib/types/champ_select.ts`):
   - Update `BuildRecommendationResult`:
     - `completed_build: ItemRecommendation[]`
     - `alternative_items: ItemRecommendation[]`
3. Frontend UI (`src/lib/components/history/MayhemBuildEnginePanel.svelte`):
   - Refactor 5-item build path: icons at `h-11 w-11`, item name, WR, lift/games, and synergy explanation underneath.
   - Add Alternative Options section underneath the 3-column grid as a sibling within the panel.
   - Show icons, name, WR, lift/synergy tags, and engine reasoning badges.
4. Update `MayhemChampSelectView.svelte` and `LiveOverlay` sprite mapping to map `completed_build.map(i => i.item_id)`.
5. Run tests & type checks.
