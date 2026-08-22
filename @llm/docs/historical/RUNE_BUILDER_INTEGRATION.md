# Rune Builder & Coachless.gg Telemetry Integration

**Date**: August 2026  
**Status**: Implemented & Verified  
**Scope**: Modular Rune Statistics Provider, Coachless.gg API Integration, Champion Roster & Name Detection, and Flowbite Card-Based UI.

---

## 1. Executive Summary

This feature replaces the legacy single-container rune editor with a modern **Card-Based Rune Builder UI** powered by real-time telemetry from **Coachless.gg** (`https://api.coachless.gg/api`). The system is architected around a provider-agnostic trait (`RuneStatsProvider`), allowing seamless pluggability for future data sources (e.g. Lolalytics, OP.GG, U.GG).

---

## 2. Telemetry API Specifications (Coachless.gg)

All endpoints consume JSON `POST` requests and return structured match telemetry:

### A. Endpoints
1. `POST /Rune/GetKeystoneData`: Returns all viable keystones across paths, overall match occurrence counts, Win Percentage Added (WPA delta), and in-game effect metrics.
2. `POST /Rune/GetRunesForKeystoneAndTree`: Returns 3 tier rows (`rowOnes`, `rowTwos`, `rowThrees`) for a specific tree, conditioned on the chosen keystone.
3. `POST /Rune/GetSecondaryTreePlaycount`: Returns percentage playcount share for candidate secondary trees (e.g. Inspiration 80.1% vs Domination 19.1%).
4. `POST /Rune/GetShardsForKeystoneAndTree`: Returns 3 rows of stat shards (`offense`, `flex`, `defense`) with occurrence counts and WPA.

### B. Common Filters Payload
```json
{
  "patch": { "major": 16, "patch": 16, "patchAdditions": 0 },
  "championIds": [64],
  "matchupChampionIds": null,
  "leagueTiers": [4, 5, 6, 7],
  "regions": null,
  "role": 1
}
```

### C. Mappings & Constants
- **League Tiers (Multi-select)**:
  - `0`: Iron, `1`: Bronze, `2`: Silver, `3`: Gold, `4`: Platinum, `5`: Emerald, `6`: Diamond, `7`: Master+
- **Roles / Lanes**:
  - `0`: Top, `1`: Jungle, `2`: Mid, `3`: ADC, `4`: Support, `5`: All Roles
- **Stat Shard IDs**:
  - Offense: `5005` (Attack Speed), `5008` (Adaptive Force), `5007` (Ability Haste)
  - Flex: `5008` (Adaptive Force), `5010` (Move Speed), `5001` (Scaling HP)
  - Defense: `5001` (Scaling HP), `5011` (Flat HP), `5013` (Tenacity / Slow Resist)

---

## 3. Architecture & Modular Rust Backend

### A. Modular Stats Provider Trait (`src-tauri/src/services/rune_stats/mod.rs`)
```rust
#[async_trait::async_trait]
pub trait RuneStatsProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    async fn get_keystones(&self, query: &BuilderQuery) -> Result<Vec<EnrichedKeystoneOption>, String>;
    async fn get_tree_runes(&self, main_tree: i64, tree_to_load: i64, keystone: Option<i64>, query: &BuilderQuery) -> Result<TreePerksStats, String>;
    async fn get_secondary_tree_playcounts(&self, tree: i64, keystone: Option<i64>, query: &BuilderQuery) -> Result<Vec<SecondaryTreeOption>, String>;
    async fn get_shards(&self, keystone: Option<i64>, query: &BuilderQuery) -> Result<StatShardsStats, String>;
    async fn get_builder_overview(&self, query: &BuilderQuery) -> Result<RuneBuilderOverview, String>;
}
```

### B. Coachless Provider (`src-tauri/src/services/rune_stats/coachless.rs`)
- In-memory 5-minute TTL caching layer.
- Parallel async requests via `tokio::join!`.
- Full CDragon metadata enrichment (perk names and local base64 icon data URIs).

### C. Champion Discovery & Name Detection (`src-tauri/src/services/champions.rs`)
- Caches CommunityDragon `champion-summary.json`.
- `detect_champion_from_name(name)`: Automatically parses rune page names to extract champion associations (e.g. *"Aatrox Conqueror"* $\rightarrow$ Aatrox `#266`) or defaults to *"All Champions"* (`ID 0`).
- `get_saved_champion_roster()`: Queries SQLite `RuneStore` for champions with saved custom runes.

---

## 4. Frontend Architecture & Two-Card Layout (Svelte 5 + Flowbite)

### A. View Mode vs. Edit Mode Workflow
- **View Mode (Default)**: Inspect saved companion pages or live League client pages with full telemetry metrics and WPA badges without accidental modifications. Header displays champion avatar / asterisk `*` next to the indicator dot and provides "Edit Page" and "Export to Game" buttons.
- **Edit Mode**: Activated via "Edit Page", enabling interactive primary tree switching, keystone picking, perk modifications, "Recommended" build application, and "Save Page" persistence.

### B. Two-Card Matrix Architecture
1. **Left Card (`PrimaryPathCard.svelte`)**:
   - **Primary Tree Selector**: 5 style icons (Precision, Domination, Sorcery, Inspiration, Resolve) at the top.
   - **Keystones Section**: Only displays the 3-4 keystones belonging to the selected primary tree in their **natural in-game CDragon order** with full keystone names, games count, and bi-directional WPA bars.
   - **Primary 3 Tiers**: 3 tier rows with authentic row names (*Heroism*, *Legend*, *Combat*, etc.), each with 3 individual perk cards (`RuneCard.svelte`) in natural in-game order.
2. **Right Card (`SecondaryPathCard.svelte`)**:
   - **Secondary Tree Selector**: 4 secondary style icons with popularity share badges (`80.1%`).
   - **Secondary 3 Tiers**: 3 tier rows with authentic row names (max 2 selected across tiers, clean cards).
   - **Stat Shards**: 3 rows of individual shard cards (Offense, Flex, Defense).
3. **Sidebar Search & Horizontal Carousel (`RunesSidebar.svelte`)**:
   - Champion search bar with horizontally scrollable circular champion avatars with dynamic left/right scroll arrows.
   - Classic champions (`600xxx`) filtered out.
   - "All Champions" uses the official Clash autofill icon.
   - List items position the champion avatar directly to the right of the keystone/style icon.
4. **Perk & Telemetry Visual Overhaul**:
   - Borderless circular perk containers where icons completely fill the space (`object-cover`).
   - Raw games count displayed with white text and opacity scaled by pick rate.
   - White pick rate bar with opacity scaled by pick rate.
   - Regular WPA text above a **bi-directional center-origin bar** (center line at `50%`, red left for negative, green right for positive, clamped at $\pm 3\%$).

---

## 5. Verification & Test Status


- `cargo test coachless`: **Passed** (0 errors)
- `svelte-check`: **Passed** (0 errors, 0 warnings)
- `vite build`: **Passed** (0 errors)

