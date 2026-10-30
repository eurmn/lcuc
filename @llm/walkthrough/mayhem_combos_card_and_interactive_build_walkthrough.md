# Mayhem Combos Card & Interactive In-Game Build Selection Walkthrough

## Overview

This update replaces the "ARAM Mayhem Augments: Catalog & Stats" card with a dedicated **Combos Card** in Champ Select (`MayhemChampSelectView.svelte`). It also introduces an **interactive build constructor** where players can assemble and customize their 6 in-game items directly from combos, recommendations, alternative items, and situational items, with automatic reactive synergy scoring.

---

## Key Changes

### 1. Dedicated Mayhem Combos Card (`MayhemCombosCard.svelte`)
- **Shape-Specific Filtering**:
  - Direct tabs allowing users to explore exact combo shapes:
    - **All Shapes**
    - **1A + 1I** (1 Augment + 1 Item, $k = 2$)
    - **1A + 2I** (1 Augment + 2 Items, $k = 3$)
    - **2A + 1I** (2 Augments + 1 Item, $k = 3$)
    - **2A + 2I** (2 Augments + 2 Items, $k = 4$)
    - **2A + 3I** (2 Augments + 3 Items, $k = 5$)
- **Synergy Prioritization**:
  - Automatically identifies combos containing the player's active augments or active build items.
  - Combos with matching elements receive a purple **Synergy** badge (`Synergy (X match)`), with an active "Synergy First" sorting toggle enabled by default.
  - Interactive sorting by WPA, Win Rate, and Total Sample Games.
- **Interactive Component Elements**:
  - Every component in the combo chain is rendered with official Mayhem tier styling (Silver, Gold, Prismatic) or item borders.
  - Hovering reveals rich `GameEntityPopover` tooltips with dynamic scaling and rules scrubbed.
  - Clicking an augment component invokes `onSelectAugment`, immediately adding it to the next empty augment picker slot.
  - Clicking an item component invokes `onSelectItem`, adding it to the targeted or next empty build slot.

### 2. Backend Apriori Combo Mining for 4-Tuples and 5-Tuples (`src-tauri/src/agent/mod.rs`)
- **Apriori-Pruned Higher-Order Mining**:
  - Extended combo aggregation to mine targeted 4-tuples ($2\text{ augments} + 2\text{ items}$) and 5-tuples ($2\text{ augments} + 3\text{ items}$).
  - Uses `combo_frequent_pairs` as a pruning dictionary: prior to counting higher-order tuples, all constituent pairs ($C(4, 2) = 6$ pairs for 4-tuples; $C(5, 2) = 10$ pairs for 5-tuples) must meet the minimum frequency threshold.
  - Avoids combinatorial explosion and completes mining within bounded execution budgets.
- **Enriched Statistics DTOs (`src-tauri/src/services/history.rs`)**:
  - Existing enrichment parser splits combo keys by `+` and resolves each `augment:` or `item:` prefix into complete `ComboComponentDto` metadata (name, icon, rarity, price, description).

### 3. Interactive In-Game Build Slots (`MayhemChampSelectView.svelte`)
- **6 Active Build Slots Grid**:
  - Replaces static item display with 6 interactive slots:
    - **Populated Slots**: Display item icon, slot order badge, price, WR, and WPA. Hovering shows a red remove button (`×`) to clear the slot.
    - **Empty Slots**: Render dashed placeholder with a plus icon, allowing players to build up to 6 core items.
    - **Slot Targeting**: Clicking any slot targets it (indicated by an amber ring and highlight), directing the next selected item into that specific slot.
  - **Reset Defaults**: A quick "Reset Defaults" action allows reverting custom item picks back to the recommended strategy baseline.
- **Interactive Multi-Source Item Selection**:
  - Clicking items in the **Combos Card**, **Alternative Items**, or **Situational Items** places them directly into the targeted or next empty build slot.
  - **Starter Item Interaction**: The starter item card is clickable to add into the build, with visual checkmarks and `In Build` badges.
  - **Visual In-Build State**: Alternative and situational items indicate whether they are already in the active build with emerald indicators and badges.
- **Full Reactivity**:
  - Selecting augments updates the backend-recommended build strategy.
  - Selecting items updates the active build items list, which triggers instant re-scoring of the Combos Card to push synergizing combinations to the top.
  - Exporting the build to the League Client (`exportMayhemBuild`) respects user-customized items.

---

## File Changes Summary

| File | Change | Description |
|---|---|---|
| `src-tauri/src/agent/mod.rs` | Modified | Added 4-tuple (2A+2I) and 5-tuple (2A+3I) Apriori combo mining with pairwise frequency validation |
| `src/lib/components/history/MayhemCombosCard.svelte` | Created | Combos card with shape filtering tabs, synergy scoring, component popovers, and click handlers |
| `src/lib/components/history/MayhemChampSelectView.svelte` | Modified | Replaced `MayhemAugmentsCard` with `MayhemCombosCard`, added 6 interactive build slots and multi-source item picking |
| `src/lib/components/history/MayhemAugmentsCard.svelte` | Removed | Replaced by `MayhemCombosCard` |

---

## Verification

- **Combo Mining**: Validated tuple generation logic and frequency pruning in `agent/mod.rs`.
- **Shape Tabs**: Verified tabs for `All Shapes`, `1A+1I`, `1A+2I`, `2A+1I`, `2A+2I`, and `2A+3I` correctly filter combos based on component count and kind.
- **Component Interactivity**: Verified clicking augment components populates the augment picker, and clicking item components populates build slots.
- **Slot Targeting**: Verified targeting an empty or filled slot replaces that exact slot.
- **Client Export**: Verified custom build items are exported to client item sets.
