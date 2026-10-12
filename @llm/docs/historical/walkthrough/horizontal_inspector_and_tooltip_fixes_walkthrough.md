# Walkthrough: Dynamic Tooltip Formula Resolutions & Top Horizontal Inspector Grid

## Overview
1. **Dynamic Tooltip Formulas & Tracker Removal**:
   - Fixed unexpanded template macros like `{{Kiwi_PressureCooker_Tooltipf4}}` by resolving fallback descriptions, quest milestones, and macro references from CommunityDragon's string tables.
   - Evaluated `mSpellCalculations` from game bin data so variables like `@DamagePerBolt@` in **Fan the Hammer** now accurately compute and render as `8 - 50.5 (+14% AD)`.
   - Stripped out unpopulated in-match runtime tracker lines such as `Damage Dealt: f2`, `Shields Generated: @f2@`, and in-sentence dynamic counters `(@f1@)`.
2. **Horizontal Inspector Grid Above Table**:
   - Transformed the build inspector from a cramped vertical right-side sidebar into a responsive horizontal grid (`grid-cols-1 md:grid-cols-2 xl:grid-cols-4`) positioned directly on top of the build performance table.
   - Both the inspector and the table now span the full 100% width of the screen (filling the full X-axis).
   - Added an inspector dismiss button `(✕)` and smooth row toggle selection.

---

## Changes Made

### 1. Tooltip Resolution Engine
- **Bin Calculations & Macro Resolution**:
  - In `generate_augment_data.js`, implemented calculation parsing for `ByCharLevelBreakpointsCalculationPart` (level scaling), `StatByCoefficientCalculationPart` (+% AD/AP/Health scaling), and `NamedDataValueCalculationPart`.
  - Resolved dynamic templates (e.g. `{{Item_Keyword_OnHit}}` -> `On-Hit`, `{{SpellName}}` -> `Ability` / `Q/W/E Ability`, `{{Cherry_FruitsOfYourLabor_Summary}}` -> string table summary).
  - Configured fallback to `DescriptionTra` for augments with in-match dynamic quest templates (e.g. `Pressure Cooker` now displays its full Quest and Reward mechanics).
  - Regenerated [`src-tauri/src/data/augment_tooltips.json`](src-tauri/src/data/augment_tooltips.json).
- **Runtime Tracker Sanitation**:
  - In [`src/lib/utils/gameDescription.ts`](src/lib/utils/gameDescription.ts), added regex filters to strip runtime tracker sentences (`Damage Dealt: f2`, `Shields Generated`, `Immobilizing Spells Blocked`, etc.) and parenthetical dynamic values `(@f1@)`.
- **Cache Invalidation**:
  - In [`src-tauri/src/services/history.rs`](src-tauri/src/services/history.rs#L119), bumped augment cache filename to `augments_v6.json` to ensure client cache refreshes with new tooltips.

### 2. Full-Width Horizontal Inspector & Table Layout
- In [`src/lib/components/history/BuildStatisticsView.svelte`](src/lib/components/history/BuildStatisticsView.svelte):
  - Removed the vertical 2-column split (`grid-cols-[minmax(0,1fr)_minmax(300px,400px)]`).
  - Switched the outer container to `flex flex-col gap-3`.
  - Created a top horizontal card when a row is selected (`max-h-[300px] overflow-y-auto scrollbar-thin`):
    - **Card 1: Identity & What It Does**: Icon, name, tier/price, and parsed in-game effects box. Includes dismiss button.
    - **Card 2: Performance Metrics & Top Champions**: 2x2 grid of games/WR, WPA, KDA, and avg damage, plus top 4 champions by WPA.
    - **Card 3: Often Paired Items**: List of synergistic items with gold prices and WPA deltas.
    - **Card 4: Often Paired Augments**: List of synergistic augments with tier badges and WPA deltas.
  - Table now occupies 100% width below the inspector, giving full breathing room to build icons, tier badges, the new card deck hand champion preview, and win-rate columns.

---

## Verification
- Pressure Cooker tooltip verified: shows full stack burn description, Quest requirement, and Reward.
- Fan the Hammer tooltip verified: calculates `8 - 50.5 (+14% AD)` physical damage without `Damage Dealt: f2`.
- Git commit: `3fdea78`. Working tree clean.
