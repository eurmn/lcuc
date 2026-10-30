# Walkthrough: Build Card Common Augments Associated With Core Items
Date: 2026-10-27
Status: Complete.

## 1. Context & Motivation
ARAM Mayhem builds mine coherent core items for champions via `mine_build_archetypes`. The build card in `MayhemChampSelectView.svelte` previously showed recommended items and summoner spells, but omitted augments associated with that specific build. Furthermore, `top_augments` in archetypes only tracked augment IDs without pick rates or build-specific win rates.

## 2. Implementation
### Backend (`src-tauri`)
- **Model updates** (`src-tauri/src/models/history.rs`):
  - Added `#[serde(default)] pub pick_rate: f64` to `MayhemBuildAugment`.
- **Archetype augment mining** (`src-tauri/src/services/history.rs`):
  - In `mine_build_archetypes`: aggregates augments across `sub_facts` (games containing this champion and the core items). Tracks `games`, `wins`, calculates Bayesian-smoothed `win_rate`, `wpa`, and `pick_rate` (`games / sample_games`).
  - Updated `MinedArchetype.top_augments` to store `Vec<(String, usize, f64, f64, f64)>` (ID, games, WR, WPA, pick rate).
  - In `compute_mayhem_champion_build`: filters out already-requested augments, applies build-specific metrics to `MayhemBuildAugment`, and backfills up to 6 augments from general champion stats if sample is sparse.
  - Added `pick_rate` population to metric strategies.

### Frontend (`src`)
- **Type updates** (`src/lib/types/champ_select.ts`):
  - Added `pick_rate?: number` to `MayhemBuildAugment`.
- **Build card UI** (`src/lib/components/history/MayhemChampSelectView.svelte`):
  - Added "Common Augments" card directly underneath Summoner Spells in the build strategy grid.
  - Displays augment icon with rarity border, name, build pick rate, and build WPA.
  - Integrates `GameEntityPopover` on hover for rich tooltip preview.
  - Integrated `handleSelectBuildAugment`: clicking an augment when in-progress automatically selects it into the next empty slot in `MayhemAugmentPicker`.
