# Session Handoff & Architecture Context: Build Statistics, Rich Tooltips & Best Matches

**Date**: October 12, 2026  
**Session Title**: Augment Tooltips, Build Statistics Inspector, Best Matches & Expression Parser  
**Primary Modules**:
- `src/lib/components/history/BuildStatisticsView.svelte`
- `src/lib/components/common/GameEntityPopover.svelte`
- `src/lib/utils/gameDescription.ts`
- `src-tauri/src/services/history.rs`
- `src-tauri/src/data/augment_tooltips.json`
- `scratch/generate_augment_data.js`

---

## 1. Executive Summary

This session undertook a major upgrade of the match history statistics and build performance subsystem for ARAM Mayhem:
1. **Tier Identification & Filter Tabs**: Implemented an augment rarity tier column (Silver, Gold, Prismatic) and dynamic toolbar filter tabs.
2. **Rich Entity Tooltips (`GameEntityPopover`)**: Popover tooltips providing full game descriptions, stat breakdowns, item pricing, and augment tiers upon hover/focus across items and augments.
3. **Synergy Visualization ("Best Champs" Card Deck Preview)**: Added an overlapping card-deck hand preview column for the top 3 synergizing champions in the main table.
4. **Horizontal Inspector Layout & Unified "Best Matches"**: Replaced vertical stacked cards with a horizontal split-card layout. Unified all partner entities (champions, items, augments) into a 3-column "Best Matches" card with tagged metrics (`((WR)) $win/$games ($wr%) | ((WPA)) $wpa`).
5. **Systemic Description Engine & Tracker Cleanup**: Solved Riot CDragon formula rendering, `mDisplayAsPercent` calculation flags, math expressions (`*20`, `*1000`, etc.), float32 precision artifacts, and in-game live match tracker pollution.

---

## 2. Subsystem Deep-Dives & Key Changes

### A. Dynamic Rarity Tier & Augment Filters
- **Backend & Domain**:
  - `MatchHistoryAugmentInfo` enriched with `rarity: Option<String>` derived from CDragon/Kiwi metadata.
  - Serialized through DTOs (`ServerStatisticRow`) down to Svelte frontend.
- **Frontend Presentation** (`BuildStatisticsView.svelte`):
  - Tier column displays badges: Slate for Silver (`S`), Amber/Gold for Gold (`G`), Fuchsia/Purple for Prismatic (`P`).
  - Active augment tier tab filter ("All", "Silver", "Gold", "Prismatic") seamlessly filters table rows without database re-queries.
  - Multi-key sorting supports sorting directly by tier rarity score (`rarityScore`).

### B. "Best Champs" Overlapping Card Deck Hand Preview
- **Main Table Column**:
  - Replaced the ambiguous single "often paired" column with a specialized **Best Champs** synergy column.
  - Champions are rendered as 3 circular avatar icons overlapping horizontally along the X-axis (`-ml-2` relative z-index stack like a poker hand).
  - Hovering/focusing each avatar activates a rich popover showing the champion's name, games played with this build item/augment, win rate, and WPA delta.
  - Sorting key `champWpa` allows ranking items and augments by their maximum champion synergy delta.

### C. Inspector Card Layout & "Best Matches" Card
- **Previous Bottleneck**: The inspector previously stacked multiple narrow boxes vertically ("What It Does", "Often Paired Items", "Often Paired Augments", "Top Champions"), pushing the main statistics table off-screen.
- **Refactored Architecture**:
  - Converted the inspector into a compact top horizontal layout:
    - **Selected Build Details Card (`lg:col-span-4`)**:
      - Displays icon (`h-11 w-11`), title, tier/price, and category tags.
      - In-game description sits directly as a native child with clean typography.
      - Performance metric bar directly underneath: Record, WR, pure game counts, WPA delta, KDA ratio, and Average Damage.
    - **Unified "Best Matches" Card (`lg:col-span-8`)**:
      - 3 equal columns: **Champions**, **Items**, and **Augments**.
      - Zero outer column gap with seamless internal dividers (`divide-x divide-gray-800/60`).
      - Entity name on top with price/tier tags; metrics formatted below:
        `((WR)) $win/$games ($wr_percentage) | ((WPA)) $wpa`.
      - `((WR))` and `((WPA))` are rendered as dark micro-badge tags (`bg-gray-800/90 border border-gray-700/60 px-1 py-0.5 text-[8px] font-bold text-gray-300`).
  - **Table Subheader**:
    - Under "Build performance", displays:
      `WR $games_won/$games_scanned ($wr_pct) $+/-wpa` (e.g. `WR 4,821/9,500 (50.7%) +0.7%`).

### D. Tooltip Parsing & Engine Improvements

#### 1. Formula & `mDisplayAsPercent` Support
- **Issue**: Augments like *Forged by the Master* rendered as:
  `"Your Item and Augment damage are increased by 0.44999998807907104. Bonus Damage: @f1.0@"`
- **Cause**: Riot's `GameCalculation` specifies `mDisplayAsPercent: true` for fractional ratios (0.45 = 45%). Previous extraction printed raw `mNumber` values without percent scaling.
- **Fix**: The recursive calculation evaluator (`formatCalculation`) checks `mDisplayAsPercent`. When true, formulas across all sub-parts are scaled by 100, float noise is stripped, and `%` is appended (producing clean **`45%`**).

#### 2. General Math Expression Parser
- **Issue**: Augments like *Firebrand* rendered as:
  `"...dealing @PercentMaxHPDamagePer5Seconds*20@% max Health magic damage..."`
- **Cause**: The previous parser only matched literal `*100`. It failed on custom scaling factors like `*20`, `*1000`, `*-100`, or `/2`.
- **Fix**: Implemented a dynamic arithmetic expression parser:
  `/@([a-zA-Z0-9_]+)(?:([*+/-])(-?[0-9]+(?:\.[0-9]+)?))?@(%?)/g`
  It dynamically resolves `valMap` values (e.g. `0.02`), applies the operator (`* 20`), cleans decimal precision (`0.4`), and appends `%` cleanly without duplicating trailing template percents (`0.4%`).

#### 3. Live Match Tracker Scrubbing
- **Issue**: Augments contained in-game scoreboard trackers:
  `Bonus Damage: @f1.0@`, `Gold Generated: <b>@f1@</b>`, `Damage Dealt: f2`.
  Deleting `@f1@` previously left orphaned labels like `Gold Generated:` hanging at the end of tooltips.
- **Fix**:
  - Scrubbed `<rules>` tracker containers.
  - Scrubbed parenthetical live counters `(@f1@)`.
  - Built a comprehensive scanner covering 30+ Riot tracking metrics (`Damage Dealt`, `Healing Done`, `Shields Generated`, `Gold Generated`, `Health Drained`, `Stacks Gained`, `Charges Refunded`, `Current Conversion`, etc.).
  - Scrubbed any hanging tracker labels ending with `:` at the end of tooltips.

#### 4. Float32 Precision Sanitization
- Normalized IEEE-754 single-precision float artifacts across all tooltips:
  `0.07999999821186066` -> `0.08`
  `0.007499999832361937 * 100` -> `0.75%` (previously erroneously rounded to `1%`).

#### 5. Defense-in-Depth Pipeline
- Applied both in the offline data pipeline ([`scratch/generate_augment_data.js`](scripts/scratch)) and the client runtime parser ([`src/lib/utils/gameDescription.ts`](src/lib/utils/gameDescription.ts)).
- Bumped backend cache key to `augments_v8.json` in [`src-tauri/src/services/history.rs`](src-tauri/src/services/history.rs#L119).

---

## 3. Repository File Map for Session

| File Path | Description |
|---|---|
| [`src/lib/components/history/BuildStatisticsView.svelte`](src/lib/components/history/BuildStatisticsView.svelte) | Main build statistics view, horizontal inspector, Best Matches 3-column layout, and Best Champs deck preview. |
| [`src/lib/components/common/GameEntityPopover.svelte`](src/lib/components/common/GameEntityPopover.svelte) | Reusable hovercard popover component for items, augments, and champions. |
| [`src/lib/utils/gameDescription.ts`](src/lib/utils/gameDescription.ts) | Runtime HTML parser for Riot item/augment descriptions and stat tags. |
| [`src-tauri/src/services/history.rs`](src-tauri/src/services/history.rs) | Rust match history service, augment cache handling (`augments_v8.json`), and stats aggregation. |
| [`src-tauri/src/data/augment_tooltips.json`](src-tauri/src/data/augment_tooltips.json) | Bundled precomputed augment name, tier, and sanitized tooltip database. |
| [`scratch/generate_augment_data.js`](scripts/scratch) | Extraction script fetching CDragon `kiwi.bin.json`, `cherry-augments.json`, and `lol.stringtable.json`. |

---

## 4. Current State & Verification

- **Working Tree**: Completely clean (committed up to `790002b`).
- **Data Integrity**: All sample test cases validated:
  - *Forged by the Master*: `"Your Item and Augment damage are increased by 45%."`
  - *Combusting Interest*: `"Burn and damage-over-time on champions generate 1 - 3 + 0.75% of damage dealt as Gold."`
  - *Firebrand*: `"Your Attacks also Burn enemies, dealing 0.4% max Health magic damage per second over 5 seconds. This Burn stacks infinitely."`
- **UI Metrics Alignment**:
  - Main subheader: `WR $games_won/$games_scanned ($wr_pct) $+/-wpa`
  - Best Matches sublines: `((WR)) $win/$games ($wr_percentage) | ((WPA)) $wpa`
