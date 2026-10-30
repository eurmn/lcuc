# Walkthrough: Svelte 5 Frontend Component Modularization

The frontend codebase of **LCU Companion** has been refactored into a modular, component-driven architecture using modern Svelte 5 runes (`$state`, `$derived`, `$props`, `$effect`). All visual styles, Flowbite patterns, `@humanspeak/svelte-motion` animation timings, and IPC behaviors have been preserved with 100% fidelity.

---

## 1. Summary of Changes

### Phase 1: Rune Tree Canvas (`src/lib/components/runes/`)
- **Decomposed [`RuneCanvas.svelte`](/src/lib/components/runes/RuneCanvas.svelte)**: Reduced from 574 lines into a clean data orchestrator coordinating sub-components.
- **Created [`RuneCanvasHeader.svelte`](/src/lib/components/runes/RuneCanvasHeader.svelte)**: Encapsulates status indicator dot, champion avatar trigger with hover edit badge, title / input box, and View/Edit mode action buttons.
- **Created [`RuneFilterTray.svelte`](/src/lib/components/runes/RuneFilterTray.svelte)**: Manages role switcher tabs and rank tier pill badges inside an animated `<AnimatePresence>` expandable tray with [`Divider.svelte`](/src/lib/components/Divider.svelte).
- **Created [`RuneLoadingSkeleton.svelte`](/src/lib/components/runes/RuneLoadingSkeleton.svelte)**: Dedicated telemetry loading placeholder with pulse spinner.
- **Created [`TelemetryStats.svelte`](/src/lib/components/runes/TelemetryStats.svelte)**: Reusable telemetry visualizer rendering pick rate game counts, white pick rate bar, divider, and bi-directional center-origin WPA delta bar with green/red indicator. Reused across [`KeystoneCard.svelte`](/src/lib/components/runes/KeystoneCard.svelte), [`RuneCard.svelte`](/src/lib/components/runes/RuneCard.svelte), and [`StatShardsCards.svelte`](/src/lib/components/runes/StatShardsCards.svelte).
- **Decomposed [`PrimaryPathCard.svelte`](/src/lib/components/runes/PrimaryPathCard.svelte)**:
  - [`PrimaryStyleSelector.svelte`](/src/lib/components/runes/PrimaryStyleSelector.svelte): 5 primary style icon buttons.
  - [`KeystonesRow.svelte`](/src/lib/components/runes/KeystonesRow.svelte): Dynamic column keystone grid.
  - [`PerkTierList.svelte`](/src/lib/components/runes/PerkTierList.svelte): 3 primary perk tier rows with authentic in-game row names.
- **Decomposed [`SecondaryPathCard.svelte`](/src/lib/components/runes/SecondaryPathCard.svelte)**:
  - [`SecondaryStyleSelector.svelte`](/src/lib/components/runes/SecondaryStyleSelector.svelte): 4 candidate style buttons with popularity % badges.
  - [`SecondaryTierList.svelte`](/src/lib/components/runes/SecondaryTierList.svelte): 3 secondary tier rows.
  - [`StatShardsCards.svelte`](/src/lib/components/runes/StatShardsCards.svelte): Offense, flex, and defense shard options.
- **Deleted 9 Unreferenced Legacy Files**: Removed dead prototype files (`BuilderFilterBar`, `ChampionRoster`, `KeystoneCardsSection`, `KeystoneSlot`, `PrimaryTreeCards`, `SecondaryTreeCards`, `StatModsSection`, `StyleSection`, `TierSlot`).

---

### Phase 2: Sidebar (`src/lib/components/sidebar/`)
- **Decomposed [`RunesSidebar.svelte`](/src/lib/components/sidebar/RunesSidebar.svelte)**: Reduced from 521 lines into a clean layout rail coordinating Profile, Champion Filter, and Dual Lists.
- **Created [`ChampionFilterSection.svelte`](/src/lib/components/sidebar/ChampionFilterSection.svelte)**: Manages collapsible search input and champion selector row with smooth layout transitions.
- **Created [`ChampionCarousel.svelte`](/src/lib/components/sidebar/ChampionCarousel.svelte)**: Horizontal scrollable champion icons list with left/right scroll buttons, favorite stars, search results, "All Champions" toggle, and active rune mismatch indicators.
- **Created [`SavedRuneItem.svelte`](/src/lib/components/sidebar/SavedRuneItem.svelte)**: Isolated motion-animated saved runeset card with primary style icon, champion avatar with hover edit pencil, active badge, and delete button.
- **Created [`ClientRuneItem.svelte`](/src/lib/components/sidebar/ClientRuneItem.svelte)**: Isolated motion-animated live client page card with style icon, all-champions avatar, active indicator, and import action.
- **Created [`SidebarSkeleton.svelte`](/src/lib/components/sidebar/SidebarSkeleton.svelte)**: Pulse placeholder for sidebar loading state.

---

## 2. Component Hierarchy Map

```text
src/
├── lib/
│   ├── components/
│   │   ├── TitleBar.svelte
│   │   ├── ProfileCard.svelte
│   │   ├── ChampionSelectModal.svelte
│   │   ├── Divider.svelte
│   │   ├── sidebar/
│   │   │   ├── RunesSidebar.svelte
│   │   │   ├── ChampionFilterSection.svelte
│   │   │   ├── SelectedChampionSlot.svelte
│   │   │   ├── ChampionCarousel.svelte
│   │   │   ├── SidebarSkeleton.svelte
│   │   │   ├── SavedRunesList.svelte
│   │   │   ├── SavedRuneItem.svelte
│   │   │   ├── ClientRunesList.svelte
│   │   │   └── ClientRuneItem.svelte
│   │   └── runes/
│   │       ├── RuneCanvas.svelte
│   │       ├── RuneCanvasHeader.svelte
│   │       ├── RuneFilterTray.svelte
│   │       ├── RuneLoadingSkeleton.svelte
│   │       ├── PrimaryPathCard.svelte
│   │       ├── PrimaryStyleSelector.svelte
│   │       ├── KeystonesRow.svelte
│   │       ├── PerkTierList.svelte
│   │       ├── SecondaryPathCard.svelte
│   │       ├── SecondaryStyleSelector.svelte
│   │       ├── SecondaryTierList.svelte
│   │       ├── StatShardsCards.svelte
│   │       ├── KeystoneCard.svelte
│   │       ├── RuneCard.svelte
│   │       └── TelemetryStats.svelte
│   ├── types/
│   └── utils/
├── App.svelte
├── main.ts
└── styles.css
```

---

## 3. Verification & UI Fidelity

- **Zero Layout Shift**: All cards, badges, and button sizes match the Flowbite and Tailwind CSS design specifications in [`@llm/docs/ui/UI.md`](/@llm/docs/ui/UI.md).
- **Motion Integrity**: Smooth physics transitions via `@humanspeak/svelte-motion` on sidebar expanding/collapsing, filter tray unfolding, and list item layout changes remain fully active.
- **Documentation Updated**: Both [`@llm/docs/ui/UI.md`](/@llm/docs/ui/UI.md) and [`GEMINI.md`](/GEMINI.md) have been synchronized with the new component hierarchy.
