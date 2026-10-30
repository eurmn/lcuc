# Task Board: task-mayhem-build-engine-main-ui

## Task Details
- **Task ID**: `task-mayhem-build-engine-main-ui`
- **Task Name**: Clean Legacy Build Advice & Build Dedicated UI for Mayhem Build Engine
- **Status**: In Progress
- **Created**: 2026-10-29

---

## Context & User Goal
The user requested:
"clean out the old implementation and make the lcuc tauri window be an UI for all the data that is being calculated for builds now."
"we need 2 frontend agents, they can discuss with eachother on what to do and separate tasks internally and wait for responses."

The modular Mayhem Build Engine subprocess (`lcu-build-engine`) calculates rich, empirical bitset mining data (`BuildRecommendationResult`):
- `recommended_boots`: Boot choice with Bayesian win rate, lift, and sample size (or `None` if owned).
- `next_items`: Top 2 next items with Bayesian win rate, synergy lift, sample size, and reasons.
- `completed_build`: Full 6-item target build path greedily optimized for synergy.
- `active_combos`: All active item-item and item-augment synergies with win rate, WPA, and sample size.
- `stats`: Engine compute latency (ms), match count, and cache hit state.

Currently, `MayhemChampSelectView.svelte` is a monolithic 2,058-line file that still contains the legacy implementation:
- Legacy `getMayhemChampionBuild` (`api_mayhem_champion_build` heuristic)
- Legacy strategy dropdown (`MayhemBuildStrategy`), `avg_metric_value`, etc.
- Legacy `LivePurchaseAdvisor.svelte` (`compute_live_build_advice`)
- Legacy `StagePathingTree.svelte` (`stages`)
- Legacy alternative & situational item lists

---

## Two Frontend Agents Collaboration Plan

The work is split between two collaborating frontend specialists:

### Agent 1: Frontend UI/Layout Architect (`@worker-frontend-architect`)
- **Focus**: Overall page layout, views decomposition, navigation, team roster & champion selection bar, live game state bridge, and deprecation/removal of legacy structures.
- **Ownership Scope**:
  - `src/lib/components/history/MayhemChampSelectView.svelte` (or orchestrating container)
  - Layout integration with `src/App.svelte`
  - Removal/cleanup of legacy components (`LivePurchaseAdvisor.svelte`, `StagePathingTree.svelte`)
- **Protocol**: Post proposed architectural breakdown and component contracts below, then sync with Agent 2.

### Agent 2: Frontend Data & Build Engine Component Specialist (`@worker-frontend-data`)
- **Focus**: Dedicated build recommendation components, active combo cards, augment/item simulator, and TypeScript API/type cleanliness.
- **Ownership Scope**:
  - `src/lib/components/history/MayhemBuildEnginePanel.svelte` (or dedicated subcomponents for recommendations, full build path, boots)
  - `src/lib/components/history/MayhemBuildCombosList.svelte` (or refactored combos grid)
  - `src/lib/components/history/MayhemBuildSimulator.svelte` (interactive slot builder & augment simulator)
  - `src/lib/types/champ_select.ts` & `src/lib/api/champ_select.ts` (cleaning unused legacy types/endpoints, keeping build engine DTOs clean)
- **Protocol**: Coordinate with Agent 1 via this board on props/events contract, verify components with `bun run check`.

---

## Shared Discussion & Contract Notes
*(Agents post and reply here to coordinate tasks internally)*

### [@worker-frontend-architect] 2026-10-29 20:10 - Architectural Proposal & Task Separation

#### 1. Scope & Deconstruction Analysis
In `MayhemChampSelectView.svelte` (currently ~2,058 lines):
- **Legacy code to purge completely**:
  - `getMayhemChampionBuild` & `getMayhemTeamBuilds` heuristic legacy API calls
  - Legacy `LivePurchaseAdvisor.svelte` & `getLiveBuildAdvice`
  - Legacy `StagePathingTree.svelte` and `activeStrategy.stages`
  - Legacy strategy dropdown (`MayhemBuildStrategy`, `selectedStrategyId`, `archetype_1`, etc.)
  - Legacy `buildAlgorithmVersion` ("v1" | "v2") toggle
  - Legacy situational items and alternative item heuristics driven by `MayhemChampionBuildSummary`
- **Core target functionality to spotlight**:
  - **Hero Recommendations Card**:
    - Recommended Boots with Bayesian WR, lift, and sample size
    - Next 2 Recommended Items with Bayesian WR, synergy lift, and reasons
    - Greedy 6-item Completed Build
    - Engine performance stats banner (compute latency ms, match count, cached state)
  - **Active Combos & Synergy List**:
    - Item-item and item-augment synergies computed by the build engine with lift and WPA
    - Integration with existing `MayhemCombosCard.svelte` or a streamlined combos view
  - **Build & Augment Interactive Simulator**:
    - 6-item active inventory slots where users can click items to equip/unequip or targeted-slot selection
    - 4-slot Mayhem Augment Picker (`MayhemAugmentPicker.svelte`)
    - Immediate reactive re-computation of `computeMayhemBuildRecommendation` whenever augments or equipped items change
  - **Live Game / Champ Select Bridge**:
    - Roster team bar (champion swaps, team members)
    - Auto-detection of champ in live Champ Select and Live In-Game inventory (`getLiveGameState`, `lcu:champ-select-session`, `lcu:gameflow-phase`)
    - Syncing overlay via `emitOverlayState` / `syncOverlayWindow`

---

#### 2. Proposed Responsibility Split

##### **Agent 1: UI / Layout Architect (`@worker-frontend-architect`)**
- **File Ownership**:
  - `src/lib/components/history/MayhemChampSelectView.svelte` (Clean layout container)
  - Purge & delete:
    - `src/lib/components/history/LivePurchaseAdvisor.svelte` (or decommission its usage)
    - `src/lib/components/history/StagePathingTree.svelte` (or decommission its usage)
- **Deliverables**:
  1. Refactor `MayhemChampSelectView.svelte` into a streamlined orchestration container (~350–500 lines instead of 2,058 lines).
  2. Maintain LCU event hooks (`lcu:champ-select-session`, `lcu:gameflow-phase`, `lcu:live-game-state`), champion selector bar, and overlay sync dispatcher.
  3. Wire up the subcomponents provided by Agent 2 into a clean, modern grid layout.
  4. Decommission legacy heuristic calls (`getMayhemChampionBuild`, `getLiveBuildAdvice`, `LivePurchaseAdvisor`, `StagePathingTree`).

##### **Agent 2: Data & Subcomponent Specialist (`@worker-frontend-data`)**
- **File Ownership**:
  - `src/lib/components/history/MayhemBuildEnginePanel.svelte` (New dedicated recommendations component)
  - `src/lib/components/history/MayhemBuildSimulator.svelte` (New 6-slot inventory & item interaction component)
  - `src/lib/components/history/MayhemCombosCard.svelte` (Clean / refine combo synergy display)
  - `src/lib/types/champ_select.ts` & `src/lib/api/champ_select.ts` (Purge dead legacy types if no longer needed, ensure clean `BuildRecommendationResult` contracts)
- **Deliverables**:
  1. Create `MayhemBuildEnginePanel.svelte`:
     - Renders Recommended Boots, Top 2 Next Items, Greedy 6-item completed build, and engine compute stats.
     - Emits `selectItem` when user clicks an item or boot to equip.
  2. Create `MayhemBuildSimulator.svelte`:
     - Houses the 6 active build slots (user inventory / simulated inventory).
     - Provides slot clear, slot target, and item placement handlers.
     - Displays gold total, item popovers, and quick-add actions.
  3. Ensure active combo synergies from `buildEngineRec.active_combos` are cleanly displayed alongside the global combos explorer.

---

#### 3. Component IPC / Props Contract Proposal

##### `MayhemBuildEnginePanel.svelte` (Agent 2)
```typescript
interface Props {
  recommendation: BuildRecommendationResult | null;
  loading?: boolean;
  knownItemMap: Map<number, MayhemBuildItem>;
  onSelectItem?: (item: MayhemBuildItem) => void;
}
```

##### `MayhemBuildSimulator.svelte` (Agent 2)
```typescript
interface Props {
  buildSlots: (MayhemBuildItem | null)[];
  selectedSlotIndex: number | null;
  onSelectSlot: (index: number) => void;
  onClearSlot: (index: number) => void;
  onItemClick?: (item: MayhemBuildItem, index: number) => void;
}
```

---

#### 4. Next Step
Waiting for Agent 2 (`@worker-frontend-data`) feedback / acknowledgment on this contract and file division before proceeding to implementation.

---

### [@worker-frontend-data] 2026-10-29 20:25 - Contract Confirmation & Enhancements
Confirmed responsibility split and architecture plan.

**Component Enhancements Agreed**:
1. `MayhemBuildEnginePanel.svelte`:
   - Props:
     ```typescript
     interface Props {
       recommendation: BuildRecommendationResult | null;
       loading?: boolean;
       knownItemMap?: Map<number, MayhemBuildItem>;
       knownAugmentMap?: Map<string, MayhemBuildAugment>;
       onSelectItem?: (item: MayhemBuildItem) => void;
     }
     ```
   - Integrates `GameEntityPopover.svelte` for boots, next priority items, and completed 6-item build path.
   - Fallback to CDragon icons (`getItemIconUrl(itemId)`) and graceful item name display if `knownItemMap` does not have metadata for an item.
   - Displays Active Combos & Synergy Pairs computed by the engine (`recommendation.active_combos`), with badges, item/augment icons, WR and WPA.
   - Displays Engine Performance Stats banner (latency in ms, match sample count, cache hit badge).

2. `MayhemBuildSimulator.svelte`:
   - Props:
     ```typescript
     interface Props {
       buildSlots: (MayhemBuildItem | null)[];
       selectedSlotIndex: number | null;
       starterItem?: MayhemBuildItem | null;
       onSelectSlot: (index: number) => void;
       onClearSlot: (index: number) => void;
       onItemClick?: (item: MayhemBuildItem, index: number) => void;
       onReset?: () => void;
     }
     ```
   - Renders 6 interactive slots with target indicator, hover clear button, item popovers, slot badges, and total gold calculation.

3. Type and API maintenance in `src/lib/types/champ_select.ts` and `src/lib/api/champ_select.ts`.

Proceeding to create `MayhemBuildEnginePanel.svelte` and `MayhemBuildSimulator.svelte`.

---

## Sub-Board Log Entries
### [@worker-frontend-architect] 2026-10-29 20:10
- Inspected `MayhemChampSelectView.svelte` and identified legacy dependencies (`getMayhemChampionBuild`, `LivePurchaseAdvisor`, `StagePathingTree`, strategy selectors, legacy stages/situational items).
- Formulated modular architecture plan splitting layout container vs dedicated build engine / simulator components.
- Posted proposed component contracts and task separation for Agent 2 on the sub-board. Awaiting synchronization.

### [@worker-frontend-data] 2026-10-29 20:30
- Confirmed contract and implemented dedicated components:
  1. `src/lib/components/history/MayhemBuildEnginePanel.svelte`:
     - Displays Recommended Boots with WR, Lift, sample games, and reasoning.
     - Displays Top 2 Next Items with WR, Lift, sample size, and ranking badges.
     - Displays Full 6-Item target build path numbered 1-6 with item icons.
     - Displays Engine performance banner (computation latency in ms, match sample count, and CACHE HIT badge).
     - Displays Active Combos & Synergy Pairs with dual item/augment icons, WR, WPA, and sample size.
     - Full `GameEntityPopover` hover integration and CDragon fallback icons.
     - Emits `onSelectItem(item)`.
  2. `src/lib/components/history/MayhemBuildSimulator.svelte`:
     - Displays 6 interactive inventory slots with targeted indicators, hover clear button, item popovers, slot numbers, and gold totals.
     - Emits `onSelectSlot`, `onClearSlot`, `onItemClick`, and `onReset`.
  3. Cleaned and verified types in `src/lib/types/champ_select.ts` and `src/lib/api/champ_select.ts`.
- Verified with `bun run check`: 0 errors, 0 warnings.
- Ready for Agent 1 (`@worker-frontend-architect`) to integrate into `MayhemChampSelectView.svelte`.

### [@worker-frontend-architect] 2026-10-29 20:45
- Created `src/lib/components/history/MayhemRosterBar.svelte`:
  - Renders Allied Team cards with local player "YOU" badge, inspected champion styling, and swap/inspect handlers.
  - Renders Available Bench champions with quick-swap triggers.
- Completely rewrote `src/lib/components/history/MayhemChampSelectView.svelte` (~470 lines down from 2,058 lines):
  - Purged all legacy imports (`getMayhemChampionBuild`, `getMayhemTeamBuilds`, `getLiveBuildAdvice`, `LivePurchaseAdvisor`, `StagePathingTree`, `MayhemChampionBuildSummary`, `MayhemBuildStrategy`).
  - Integrated `MayhemRosterBar`, `MayhemBuildEnginePanel`, `MayhemBuildSimulator`, `MayhemAugmentPicker`, `MayhemCombosCard`, and `MayhemEncyclopediaModal`.
  - Bound state and reactivity: `computeMayhemBuildRecommendation` executes reactively on changes to champion, simulated items, or picked augments.
  - Forwarded overlay state via `emitOverlayState` (`recommendedBoots`, `nextItemIds`, `completedBuild`, `buildEngineRec`).
  - Connected live game state (`live:game-state`) to auto-populate inventory slots and synchronize current champion.
  - Clean two-column grid (`xl:col-span-7` recommendation & simulator + `xl:col-span-5` combos & synergies).
- Verified with `bun run check`: 0 errors, 0 warnings across the workspace.
- Task complete.

