# LCU Companion Mayhem UI Data Flow Walkthrough

This document maps the complete data flow from Rust backend through to Svelte UI rendering for all Mayhem-related features. It serves as a reference for understanding how data moves through the system and where the MCP bridge can inspect each layer.

---

## 1. Item Build Computation

### Rust Functions
- `src-tauri/src/services/history.rs:4726` — `compute_mayhem_champion_build(champion_id, requested_augments, requested_items, algorithm_version)`
- `src-tauri/src/services/history.rs:4451` — `compute_mayhem_champion_build_v2(...)` (primary version used)
- `src-tauri/src/services/history.rs:4912` — call site for v2

### Data Flow
1. **Entry point**: Tauri command `api_mayhem_champion_build` (champ_select.rs:85) or `api_mayhem_team_builds` (champ_select.rs:99).
2. **Filter items**: `requested_items` are filtered via `item_catalog::is_legendary_item(*id as u32)` — only fully completed legendaries count; components (Long Sword, etc.) are excluded.
3. **Cache key**: `(champion_id, normalized_augments.clone(), normalized_items.clone(), version.clone())` — items are part of the cache key so the build is conditioned on owned legendaries.
4. **V2 processing** (`compute_mayhem_champion_build_v2`):
   - Parses facts via `parse_facts_for_v2(facts, items_catalog)`.
   - **Item-conditioned stats**: If `normalized_items` is non-empty, only facts whose legendaries include all requested items are kept. If fewer than 15 games remain after filtering, the item filter is dropped, then augments are dropped one-by-one until >= 15 games remain. This avoids noise from tiny samples.
   - Augment map is built from the filtered sample, then cached in `MAYHEM_CHAMPION_AUGMENTS_CACHE` with key `(champion_id, normalized_augments, normalized_items)`.
   - Baseline WR/WPA computed from the same conditioned sample.
5. **Result**: `MayhemChampionBuildSummary` returned to frontend via Tauri command response.

### IPC Commands (MCP bridge)
- `ipc_execute_command` with `api_mayhem_champion_build` — invoke with `champion_id`, `augment_ids`, `item_ids` (Vec<i64>), `algorithm_version` to see the full summary DTO.
- `ipc_get_backend_state` — inspect cache contents (`MAYHEM_CHAMPION_AUGMENTS_CACHE`, `MAYHEM_CHAMPION_BUILDS_CACHE`).

### Svelte Rendering
- `src/lib/components/history/MayhemBuildEnginePanel.svelte` — receives `MayhemChampionBuildSummary` DTO and renders:
  - Recommended build items
  - Performance metrics bar
  - "Best Matches" 3-column synergy card (champions/items/augments with WR/WPA badges)
- `src/lib/components/history/MayhemCombosCard.svelte` — combo/synergy visual reference.

---

## 2. Augment Selection & Cards

### Rust Functions
- `src-tauri/src/services/live_overlay.rs:939` — `scan_title_band_from_image(img, champion_id)` — OCR-based scan of the 3-card augment selection area.
- `src-tauri/src/services/live_overlay.rs:997` — `scan_in_game_augment_cards(champion_id, with_augment_ids)` — F8 hotkey scan.
- `src-tauri/src/services/history.rs:6629` — `enrich_overlay_augment_cards(champion_id, cards, owned_item_ids)` — enriches raw OCR cards with statistics and synergy data.
- `src-tauri/src/services/history.rs:6077` — `get_mayhem_augments(query)` — handles `with_item_ids` query param, filters to legendaries only.

### Data Flow (F8 scan path)
1. **LCU live state**: `fetch_live_game_state()` reads player's current inventory from the LCU API (`live_game.rs`).
2. **`current_legendary_inventory_ids()`** (live_overlay.rs:941): reads `state.items`, filters via `item_catalog::is_legendary_item(*id as u32)` — components like Long Sword are excluded; only fully completed legendaries count.
3. **OCR scan**: `scan_title_band_from_image` or `scan_in_game_augment_cards` captures screen region, runs OCR, gets augment IDs.
4. **Query augmentation**: `get_mayhem_augments(MayhemAugmentsQuery { champion_id, page_size: Some(1000), with_augment_ids, with_item_ids: Some(inventory_ids) })` — passes the player's current legendary inventory as `with_item_ids`.
5. **Enrichment**: `enrich_overlay_augment_cards(champion_id, matches, &inventory_ids)`:
   - For each augment card, calls `find_top_legendary_items_for_augment(index, augment_id_or_name, 6)` to get 6 synergy candidates.
   - **Removes owned items**: `top_synergies.retain(|r| !owned_item_ids.contains(&(r.item_id as i64)))` — items the player already owns are excluded from suggestions.
   - Keeps top 2 synergies.
   - Calls `get_mayhem_augments` again (with item/augment filters) to get statistics.
   - Returns `OverlayAugmentCard` DTOs with `win_rate`, `games`, `wpa`, `tooltip`, `synergy_items`, `combos`.

### IPC Commands (MCP bridge)
- `ipc_execute_command` with `api_scan_live_augment_cards` — invoke with `championId`, `withAugmentIds` to trigger a scan and see the raw OCR results.
- `ipc_execute_command` with `api_mayhem_augments` — query with `with_augment_ids` and `with_item_ids` to see the augmented items response.
- `webview_screenshot` / `webview_dom_snapshot` — capture the overlay UI to verify card rendering, synergy items, and badge placement.

### Svelte Rendering
- `src/lib/components/overlay/LiveMayhemOverlay.svelte`:
  - `loadBuildData(champId, augs)` — calls `getMayhemChampionBuild(champId, augs, "v2")` and `getLiveBuildAdvice({..., inventoryItemIds: liveGameState.items.map(i => i.item_id), ..., augmentIds: augs})`.
  - `handleScanCards()` — calls `scanLiveAugmentCards(championId, augmentIds)`.
  - Displays `offeredAugments` list with win rate, games, WPA, tooltips, and synergy items.
- `src/lib/components/overlay/AugmentIcon.svelte` — individual augment icon with tier border (Silver/Gold/Prismatic).
- `src/lib/components/history/MayhemChampSelectView.svelte` — uses `scanLiveAugmentCards` in champion select view.

---

## 3. Combo / Synergy Suggestions

### Rust Functions
- `src-tauri/src/build_engine/bitset_miner.rs` — `find_top_legendary_items_for_augment(index, augment_id_or_name, limit)` — returns top N legendary items that synergize with the given augment.
- `src-tauri/src/services/history.rs:6646` — inside `enrich_overlay_augment_cards`, calls `find_top_legendary_items_for_augment` twice (by ID, then by name) and filters out owned items.

### Data Flow
1. **Index building**: `ChampionTransactionIndex` is built from historical match data (DB `matches` table, `item_ids_json`, `augment_ids_json`).
2. **Query**: Given an augment ID or name, `find_top_legendary_items_for_augment` finds items with the highest lift/WPA for that augment, limited to legendary items only.
3. **Owned-item exclusion** (in `enrich_overlay_augment_cards`): The 6 candidates have `owned_item_ids` filtered out, then top 2 remain.
4. **Result**: Two `TopSynergyItemResult` items (item_id, slot_idx, name, wpa, lift) passed into the `OverlayAugmentCard` DTO as `synergy_items`.

### IPC Commands (MCP bridge)
- `ipc_execute_command` with `api_mayhem_champion_build` or `api_mayhem_augments` — the build summary and augment response already include synergy data.
- `webview_dom_snapshot` — inspect the synergy panel in the overlay to see which items appear and their WR/WPA badges.

### Svelte Rendering
- `src/lib/components/overlay/LiveMayhemOverlay.svelte` — beneath recommended build items, the `SYNERGY ITEMS` native panel renders item icons (22px bordered sprites, row height 30) for the top synergizing items.
- `src/lib/components/common/OverlaySynergyRow` — row component with `icon_path`, tier border colors.
- `src/lib/components/history/MayhemCombosCard.svelte` — "Best Matches" 3-column card (Champions, Items, Augments) with `((WR))` and `((WPA))` micro-badges.

---

## 4. Card Stat Conditioning (WR/WPA)

### Rust Functions
- `src-tauri/src/services/history.rs:4467-4530` — inside `compute_mayhem_champion_build_v2`, the augmented sample filtering logic.
- `src-tauri/src/services/history.rs:4933-4960` — v1 filter path (also applies item membership).
- `src-tauri/src/services/history.rs:4874` — `compute_mayhem_champion_build` now accepts `requested_items` and filters via `item_catalog::is_legendary_item`.

### Conditioning Logic (v2)
1. **Full legendary set first**: Stats are conditioned on the player's picked augments **and** owned complete legendaries.
2. **15-game floor**: 
   - If `normalized_items` is non-empty and filtered sample has >= 15 games, keep it.
   - If < 15 games, drop the item filter entirely (games increase).
   - If still < 15 after dropping items, drop augments one-by-one until >= 15 games remain.
3. **Baseline**: The win rate from the final sample is used as the WR baseline; WPA is computed from damage/gold/duration aggregates on the same sample.

### Item Filtering Detail
- `is_legendary_item(id as u32)` only matches fully completed legendaries — components (Long Sword, B. F. Sword, etc.) are **excluded**.
- If boots should count toward the legendary set, switch from `is_legendary_item` to `is_completed_boot` (see `item_catalog.rs`).

### IPC Commands (MCP bridge)
- `ipc_execute_command` with `api_mayhem_champion_build` — pass `item_ids` to see how the summary changes when items are included/excluded.
- `ipc_execute_command` with `api_mayhem_augments` — query with different `with_item_ids` to see WR/WPA change.

---

## 5. MCP Bridge Integration

### Setup (already applied)
- **Dependency**: `tauri-plugin-mcp-bridge = "0.13"` in `src-tauri/Cargo.toml`.
- **Registration**: Debug-only in `src-tauri/src/lib.rs`:
  ```rust
  #[cfg(debug_assertions)]
  {
      builder = builder.plugin(tauri_plugin_mcp_bridge::init());
  }
  ```
- **Global Tauri**: `withGlobalTauri: true` in `src-tauri/tauri.conf.json`.
- **Capabilities**: `mcp-bridge:default` added to `src-tauri/capabilities/default.json`.

### Bridge Behavior
- **WebSocket server**: Binds to `0.0.0.0:9223` by default (all interfaces). For localhost-only: use `tauri_plugin_mcp_bridge::Builder::new().bind_address("127.0.0.1").build()`.
- **Available tools** (via `driver_session`):
  - `webview_dom_snapshot` — see the actual rendered DOM, not Svelte source.
  - `webview_screenshot` — full viewport screenshot (app reported 2560x1440, webview 2560x1439 CSS px).
  - `webview_find_element` / `webview_interact` — click, focus, type in the webview.
  - `ipc_execute_command` — call any Tauri command and get the JSON response.
  - `ipc_get_backend_state` — read managed state (cache maps, DB contents).
  - `ipc_emit_event` — push events into the app.

### How to use from the editor
```json
{
  "action": "execute",
  "command": "api_mayhem_champion_build",
  "args": { "champion_id": 1, "augment_ids": ["1156"], "item_ids": [3089], "algorithm_version": "v2" }
}
```
This returns the full build summary DTO, showing exactly which items/augments were conditioned.

```json
{
  "action": "execute",
  "command": "api_scan_live_augment_cards",
  "args": { "championId": 1, "withAugmentIds": ["1156"] }
}
```
Triggers an OCR scan and returns the detected augment cards with statistics.

```json
{
  "action": "screenshot"
}
```
Captures the current webview render — use to verify synergy icon placement, card alignment, overlay banner, etc.

---

## 6. Key File Map

| Purpose | File |
|---|---|
| Item catalog (is_legendary_item, is_completed_boot, is_component_item, LEGENDARY_SET/BOOT_SET/COMPONENT_SET) | `src-tauri/src/build_engine/item_catalog.rs` |
| Build engine (caching, bitset operations) | `src-tauri/src/build_engine/` |
| Mayhem champion build computation (v1/v2, conditioning) | `src-tauri/src/services/history.rs` |
| Live overlay (OCR scan, inventory, card enrichment) | `src-tauri/src/services/live_overlay.rs` |
| MCP bridge setup | `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`, `src-tauri/capabilities/default.json` |
| Svelte overlay UI | `src/lib/components/overlay/LiveMayhemOverlay.svelte` |
| Svelte build engine panel | `src/lib/components/history/MayhemBuildEnginePanel.svelte` |
| Svelte combos card | `src/lib/components/history/MayhemCombosCard.svelte` |
| Svelte augment icon | `src/lib/components/common/AugmentIcon.svelte` |
| LCU live game state | `src-tauri/src/services/live_game.rs` |
| Mayhem augment query API | `src/lib/api/champ_select.ts` |

---

## How to Inspect with MCP Bridge

1. **Start the app**: `bun run tauri dev` (confirmed running on port 9223).
2. **Connect**: `driver_session start` → `driver_session status` (connected: `Tauri App (localhost:9223)`).
3. **Inspect build data**: 
   - `ipc_execute_command` → `api_mayhem_champion_build` with various `item_ids` to see WR/WPA change.
4. **Inspect augment cards**:
   - `ipc_execute_command` → `api_scan_live_augment_cards` (requires League running, augment selection open).
   - `webview_screenshot` → verify card rendering on screen.
5. **Inspect synergy panel**:
   - `webview_dom_snapshot` → find the `SYNERGY ITEMS` panel and see which item icons are rendered.
   - `webview_interact` → click around, verify hover states, tier borders.

---

*Document generated from codebase analysis on 2026-10-31. Coverage includes all Rust data flows, Svelte presentation layers, and MCP bridge inspection points.*