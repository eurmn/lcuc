# Walkthrough: Augment Tier Column, Rich Game Descriptions & Separated Synergies

## Overview
Transformed the build performance and match history interfaces from raw numbers into an educational tool by surfacing what each item and augment actually does in-game:
1. **Dedicated Augment Tier Column**: Cleanly aligns Silver, Gold, and Prismatic tiers in their own table column with sorting support.
2. **What It Does (In-Game Descriptions & Stats)**: Highlights base stats, unique passives, actives, gold prices, and categories directly in the sidebar inspector.
3. **Interactive Hover & Focus Popovers**: Rich League-themed tooltips across the Build Statistics table, synergy lists, and Match History.
4. **Separated Synergies**: Separates "Often Paired With This" into two distinct sections: **Often Paired Items** and **Often Paired Augments**.

---

## Changes Made

### 1. Backend Data & Resolution
- **Augment Tooltips Map**:
  - Extracted 1,756 augment tooltips and parameters from CommunityDragon's `kiwi.bin.json`, `cherry-augments.json`, and `lol.stringtable.json` into [`src-tauri/src/data/augment_tooltips.json`](src-tauri/src/data/augment_tooltips.json).
  - Bundled as offline-ready static data in [`src-tauri/src/services/history.rs`](src-tauri/src/services/history.rs#L199) using `include_str!`.
- **Item Metadata Expansion**:
  - Expanded `ITEMS_CACHE` to store [`CachedItemInfo`](src-tauri/src/services/history.rs#L202-L210) including rich `description`, `price_total`, and `categories`.
- **Enrichment**:
  - In [`enrich_statistics_response`](src-tauri/src/services/history.rs#L1020-L1150) and [`enrich_global_statistics`](src-tauri/src/services/history.rs#L940-L1015), injected description, price, categories, and rarity into both primary rows and `partners`.
  - Bumped resolution cache to `augments_v5.json`.
- **Models**:
  - Updated [`MatchHistoryItemInfo`](src-tauri/src/models/history.rs#L5-L12) and [`MatchHistoryAugmentInfo`](src-tauri/src/models/history.rs#L14-L21).

### 2. Frontend Formatting & Components
- **Rich Tag Parser**:
  - Created [`src/lib/utils/gameDescription.ts`](src/lib/utils/gameDescription.ts) to parse League XML markup (`<stats>`, `<attention>`, `<physicalDamage>`, `<speed>`, `<rules>`, etc.) into semantic Tailwind CSS spans with inline stat icons.
- **Entity Popover**:
  - Created [`src/lib/components/common/GameEntityPopover.svelte`](src/lib/components/common/GameEntityPopover.svelte) with rarity glow, gold coin icons, and formatted descriptions.
- **Build Statistics**:
  - Updated [`BuildStatisticsView.svelte`](src/lib/components/history/BuildStatisticsView.svelte):
    - Added a dedicated **Tier** column when `kind === "augment"`, with sorting by rarity rank (`Prismatic` > `Gold` > `Silver`).
    - Added the **"What It Does"** inspection card in the sidebar showing parsed effects and stats.
    - Split synergies into **Often Paired Items** and **Often Paired Augments** subsections.
    - Attached `GameEntityPopover` to table rows and synergy items.
- **Match History**:
  - Updated [`MatchHistoryView.svelte`](src/lib/components/history/MatchHistoryView.svelte) with `GameEntityPopover` attached to match items and match augments.

---

## Verification & Validation

1. **Git Status & Working Tree**:
   - Staged and committed all changes. Working tree is clean.
2. **Component Integration**:
   - `GameEntityPopover` attached seamlessly with Flowbite popovers.
   - Separate synergy queries cleanly distinguish `item:*` and `augment:*` IDs.
