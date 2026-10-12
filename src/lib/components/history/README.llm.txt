# History Components Directory (`src/lib/components/history`)

## Overview
Contains Svelte 5 presentation components for viewing match history cards and deep-dive statistics projections (champions, builds, augments, compare mode).

## Components
- `ProfileStatisticsView.svelte`:
  - Renders the "Champions" statistics table and right-hand champion inspection panel.
  - Supports scopes: `"personal"` and `"compare"` (side-by-side Personal WR/WPA vs Global WR/WPA).
  - Right sidebar lists associated items and augments with icons, filtered to exclude 1-game noise, sorted by WPA descending.
  - Integrates 200ms search debouncing (`handleSearchInput`) and client-side cached reads via `statistics-data.ts`.
- `BuildStatisticsView.svelte`:
  - Renders performance tables for `"item"`, `"augment"`, and `"combo"` build aspects.
  - Right sidebar shows associated champions and companion augments/items with icons and frequency-filtered WPA ordering.
- `MatchHistoryView.svelte`:
  - Renders match history cards with game duration, date, KDA, items, and augments.
