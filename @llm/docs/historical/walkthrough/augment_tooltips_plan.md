# Augment Tooltips Plan (Rune Tooltip Scheme Architecture)

## Overview
This document outlines the architecture for introducing rich augment tooltips and hover info across the application, mirroring our established rune tooltip scheme implemented in [`RunePopover.svelte`](src/lib/components/runes/RunePopover.svelte) and [`parseRuneDescription`](src/lib/utils/runeDescription.ts).

---

## 1. Data Sourcing & Resolution
1. **Kiwi & Cherry Data Catalogs**:
   - In [`kiwi.bin.json`](https://raw.communitydragon.org/latest/game/maps/modespecificdata/kiwi.bin.json), each augment entry contains:
     - `AugmentNameId`: e.g. `"ARAM_LegDay"`
     - `AugmentTooltipTra` / `DescriptionTra`: localization key (e.g. `"Cherry_LegDay_Tooltip"`)
     - `RootSpell`: points to the spell object (e.g. `Maps/ModeSpecificData/Augments/ARAM_LegDay/Augment_ARAM_LegDay`) with calculation values (`DataValues`)
   - The localized summary descriptions and tooltips can be loaded either through:
     - The LCU client's active game-data endpoint `/lol-game-data/assets/v1/cherry-augments.json` or localized dictionary if connected.
     - Mode-specific spell description strings or CommunityDragon's localized dictionary table.
2. **Data Structure Extension**:
   - Extend [`MatchHistoryAugmentInfo`](src-tauri/src/models/history.rs) and [`ServerStatisticRow`](src/lib/api/agent.ts) to include `tooltip: Option<String>` and `summary: Option<String>`.

---

## 2. Text Parser & Color Theming
Mirroring [`parseRuneDescription`](src/lib/utils/runeDescription.ts):
- League client augment tooltips contain formatting tags:
  - `<attention>` / `<gold>` / `<physicalDamage>` / `<magicDamage>` / `<trueDamage>` / `<healing>` / `<shield>` / `<speed>` / `<status>`
- Create `src/lib/utils/augmentDescription.ts`:
  - Translates Game Data tags to Tailwind CSS color classes:
    - `<attention>` → `text-amber-300 font-semibold`
    - `<physicalDamage>` → `text-orange-400 font-semibold`
    - `<magicDamage>` → `text-cyan-400 font-semibold`
    - `<trueDamage>` → `text-stone-100 font-semibold`
    - `<healing>` / `<shield>` → `text-emerald-400 font-semibold`
    - `<status>` → `text-purple-300 font-semibold`

---

## 3. Presentation Component: `AugmentPopover.svelte`
Modeled directly on [`RunePopover.svelte`](src/lib/components/runes/RunePopover.svelte):
1. **Header**:
   - Round augment icon with tier-specific border glow:
     - Silver: `border-slate-500 shadow-[0_0_8px_rgba(148,163,184,0.4)]`
     - Gold: `border-yellow-500 shadow-[0_0_8px_rgba(234,179,8,0.5)]`
     - Prismatic: `border-fuchsia-500 shadow-[0_0_10px_rgba(217,70,239,0.6)]`
   - Augment localized name (`font-bold text-sm text-white`).
   - Augment tier pill (`Silver`, `Gold`, or `Prismatic`).
2. **Body**:
   - Parsed HTML description with `slide` transition (duration 200ms) and `triggerDelay={400}` for smooth UX.
3. **Integration Points**:
   - [`BuildStatisticsView.svelte`](src/lib/components/history/BuildStatisticsView.svelte) (when hovering augment rows or partner pills).
   - [`MatchHistoryView.svelte`](src/lib/components/history/MatchHistoryView.svelte) (when hovering match augment badges).
