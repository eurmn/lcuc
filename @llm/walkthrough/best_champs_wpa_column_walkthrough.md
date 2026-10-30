# Walkthrough: Best Champions WPA Column & Card Deck Hand Preview

## Overview
Added a new **Best Champs** column to the build statistics table (`BuildStatisticsView.svelte`) that visually highlights which champions benefit most from each item, augment, or combo.

Key Highlights:
1. **Card Deck Hand Avatar Display**: Up to 3 champion portraits rendered horizontally on the same Y-axis, overlapping with negative spacing (`-space-x-2.5`) resembling a fanned hand of cards. On container hover, cards separate slightly (`hover:space-x-0.5`), and individual champions scale up on hover (`hover:scale-125 hover:z-30`).
2. **Top WPA Metric**: The highest champion's WPA delta is rendered directly beside the card hand in green/red (e.g., `+8.5%`), providing immediate at-a-glance value comparison.
3. **Interactive Tooltips**: Hovering any champion card provides a detailed tooltip showing `${name}: +${WPA}% WPA (${games} games, ${winRate}% WR)`.
4. **Full Sorting Support (`champWpa`)**: Added `champWpa` sort key to both frontend client-side sorting and backend statistics query engine (projection loader, facts builder, and compare mode), allowing users to rank items/augments by which ones provide the highest impact to their top champion.

---

## Changes Made

### 1. Backend Statistics Engine
- In [`src-tauri/src/agent/mod.rs`](src-tauri/src/agent/mod.rs):
  - Pre-sorted `champion_rows` by WPA descending in `statistic_row()`.
  - Added support for `"champWpa"` sort key in `load_statistics()`, `load_statistics_from_facts()`, and compare mode calculations by extracting the maximum WPA value across the row's `champions` array (defaulting to `-999.0` for items without champion facts).

### 2. Frontend Build Statistics Interface
- In [`src/lib/components/history/BuildStatisticsView.svelte`](src/lib/components/history/BuildStatisticsView.svelte):
  - Added `"champWpa"` to `SortKey` type definition.
  - Implemented `getTopChampions(row, count)` to extract and sort the top 3 champions by WPA descending (tiebreaking by games played).
  - Implemented `getBestChampionWpa(row)` for reactive client-side sorting.
  - Added the **Best Champs** table header with sort indicator (`sortIndicator("champWpa")`).
  - Implemented the card deck hand avatar stack with smooth transitions, z-indexing, and hover effects in each row of `<tbody>`.

---

## Verification & Git Status
- Code staged and committed to git: `1498b78 ("feat(history): add Best Champs WPA column with card deck hand avatar preview")`.
- Working tree clean.
