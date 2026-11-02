# Mayhem Build Engine: Complete Data Flow Documentation

> **Scope**: Item builds, augment selection, combo synergies, and native overlay presentation.
> **Architecture**: Rust backend → Tauri IPC → Svelte 5 frontend → native Win32 overlay.

---

## 1. Data Origin: Rust Build Engine Subprocess

### 1.1 Binary & IPC Protocol
- **Binary**: `lcu-build-engine` (`src-tauri/Cargo.toml[[bin]]`, `src-tauri/src/bin/lcu_build_engine.rs`)
- **IPC**: Stdio JSON lines (`src-tauri/src/build_engine/ipc.rs`), monotonic correlation IDs (`u64`)
- **Supervisor**: `host.rs` spawns child with `CREATE_NO_WINDOW` (Windows), 1.5s timeout, auto-restart, in-process fallback
- **Tauri command**: `api_compute_mayhem_build_recommendation` (`src-tauri/src/commands/build_engine.rs`)

### 1.2 Request DTO (`src-tauri/src/build_engine/models.rs:49-57`)
```rust
pub struct BuildRecommendationResult {
    pub champion_id: u32,
    pub active_combos: Vec<ActiveComboSynergy>,
    pub next_items: Vec<ItemRecommendation>,
    pub recommended_boots: Option<ItemRecommendation>,
    pub completed_build: Vec<ItemRecommendation>,
    pub alternative_items: Vec<ItemRecommendation>,
    pub stats: EnginePerformanceStats,
}
```

### 1.3 Core Data Structures

| Struct | Fields | Description |
|---|---|---|
| `ActiveComboSynergy` | `combo_type`, `item_ids: Vec<u32>`, `augment_ids: Vec<String>`, `label: String`, `win_rate: f64`, `wpa: f64`, `sample_size: usize` | Item‑item or item‑augment synergies mined vertically |
| `ItemRecommendation` | `item_id: u32`, `win_rate: f64`, `lift: f64`, `sample_size: usize`, `is_boot: bool`, `reason: String` | One ranked recommendation item |
| `EnginePerformanceStats` | `load_time_ms`, `compute_time_ms`, `total_time_ms`, `match_count: usize`, `cache_hit: bool` | Backend compute metrics |

### 1.4 Item Catalog Filtering (Legendary‑Only)
- **Source**: `src-tauri/src/build_engine/item_catalog.rs` + `legendary_items.json` (439 legendaries + 25 completed boots)
- **`is_valid_build_item(id)`**: retains only legendaries (`priceTotal >= 2000` & `to.is_empty()`) and completed boots
- **Excluded**: components, starters, trinkets, Poro‑Snax (`2050`, `2052`), consumables
- **Implemented in**: `data_loader.rs:21` (`parse_items` filters via `is_valid_build_item`), `bitset_miner.rs:370‑398`, `live_overlay.rs:940‑955` (`current_legendary_inventory_ids`)

---

## 2. Tauri Command Layer

### 2.1 Entry Point
`src-tauri/src/commands/build_engine.rs:5-11`:
```rust
#[tauri::command]
pub async fn api_compute_mayhem_build_recommendation(
    champion_id: u32,
    owned_items: Vec<u32>,
    owned_augments: Vec<String>,
) -> Result<BuildRecommendationResult, String> {
    request_build_recommendation(champion_id, owned_items, owned_augments).await
}
```

### 2.2 API Wrapper (`src/lib/api/champ_select.ts:128-138`)
```typescript
export async function computeMayhemBuildRecommendation(
  championId: number,
  ownedItems: number[],
  ownedAugments: string[],
): Promise<BuildRecommendationResult> {
  return invoke<BuildRecommendationResult>("api_compute_mayhem_build_recommendation", {
    championId,
    ownedItems,
    ownedAugments,
  });
}
```

### 2.3 Overlay State Emission (`src/lib/api/champ_select.ts:154-156`)
```typescript
export async function emitOverlayState(payload: Record<string, unknown>): Promise<void> {
  return invoke<void>("api_emit_overlay_state", { payload });
}
```

---

## 3. Frontend State & Reactive Layer

### 3.1 `MayhemChampSelectView.svelte` — Orchestration Container
- **State**: `buildEngineRec: $state<BuildRecommendationResult | null>(null)`, `loadingRec`, `knownItemMap`, `knownAugmentMap`
- **Effect**: `$effect(() => { ... void updateRecommendation(champId, itemIds, augs); })` watches `selectedChampionId`, `userBuildSlots`, `augmentIds`
- **Debounced key**: `lastRecKey` prevents redundant RPC calls

### 3.2 Recommendation Computation (`MayhemChampSelectView.svelte:227-259`)
```typescript
async function updateRecommendation(champId: number, itemIds: number[], augs: string[]) {
  if (!champId || champId <= 0) { buildEngineRec = null; return; }
  const currentKey = `${champId}:${itemIds.slice().sort().join(",")}:${augs.slice().sort().join(",")}`;
  if (currentKey === lastRecKey) return;
  lastRecKey = currentKey;

  loadingRec = true;
  try {
    const rec = await computeMayhemBuildRecommendation(champId, itemIds, augs);
    buildEngineRec = rec;
    if (overlayEnabled) {
      void emitOverlayState({
        source: "main_window",
        championId: champId,
        championName: currentSelectedChampion?.name ?? selectedChampion?.name ?? "",
        strategyId: "engine_v2",
        augmentIds: augs,
        recommendedBoots: rec?.recommended_boots?.item_id,
        nextItemIds: rec?.next_items?.map((i) => i.item_id) ?? [],
        completedBuild: rec?.completed_build?.map((i) => (typeof i === "number" ? i : i.item_id)) ?? [],
        buildEngineRec: rec,
      });
    }
  } catch (err) { logger.warn("frontend.build_engine", "Failed computing recommendation", err); }
  finally { loadingRec = false; }
}
```

### 3.3 `knownItemMap` / `knownAugmentMap` Derived Maps (`MayhemChampSelectView.svelte:124-217`)
- Populate from `itemsCatalog` (Tauri `api_get_items_catalog`) + auto‑added CDragon icons for any `BuildRecommendationResult` IDs not already in catalog
- Handles `recommended_boots`, `next_items`, `completed_build`, `alternative_items`, and `active_combos`

---

## 4. Frontend UI Presentation Components

### 4.1 `MayhemBuildEnginePanel.svelte` — 3‑Column Recommendation Card
**Props**: `recommendation`, `loading`, `knownItemMap`, `knownAugmentMap`, `onSelectItem`

#### 4.1.1 Next Priority Items (top 2)
- Grid cell per item: icon (CDragon fallback), ranked badge (1/2), name, WR%, Lift, sample size (`Xg`)
- Hover `GameEntityPopover` with description (`reason` field)
- Click → `handleItemClick(itemId)` → parent `handleEquipItem`

#### 4.1.2 Completed 5‑Item Build Path
- 5‑item grid (`grid-cols-5`), each item shows:
  - **Win rate** (`XX.X%`) on top
  - Item icon (36x36, purple border) with number badge (1‑5) at bottom‑right
  - Name + `sample_size g` + `Lift` on bottom
  - `reason` snippet shown in popover
- Click → equip to simulator

#### 4.1.3 Recommended Boots
- Single boot slot (36x36, cyan border)
- WR%, Lift, games count, optional `reason` text

#### 4.1.4 Alternative Options
- Ranked candidate legendaries not in the primary build path
- 2‑4 items per row (2sm / 4lg), each with icon, WR%, Lift, reason

#### 4.1.5 Active Combos & Synergy Pairs
- Horizontal chips: item icons + augment icons (with `AugmentIcon` tier border)
- Chip shows: combo label, WR%, WPA% (`+/-XX%`), sample size (`Xg`)
- Tier borders: Silver `#a3b9bd`, Gold `#e3c49f`, Prismatic `linear-gradient(180deg, #e78fff, #70008f)`

#### 4.1.6 Engine Performance Stats Banner
- Total compute time (`Xms`), match count, CACHE HIT badge (emerald)

### 4.2 `MayhemBuildSimulator.svelte` — 6‑Slot Build Simulator
**Props**: `buildSlots`, `selectedSlotIndex`, `onSelectSlot`, `onClearSlot`, `onItemClick`, `onReset`, `starterItem?`

- 6 slots (2×3 grid), starter item optional at top
- Each slot: icon, count, price (g), pick_rate / WPA badges (optional)
- **Selection flow**: click empty slot → target highlight; click targeted slot → clear; click slot with item → `handleSlotPress` → `onItemClick`
- Emits `onSelectSlot(idx)`, `onClearSlot(idx)`, `onItemClick(item, idx)`, `onReset()`
- Gold total at top (`sum of price_total`)

### 4.3 `MayhemCombosCard.svelte` — ARAM Mayhem Combo Synergies (Separate Tab)
**Props**: `championId`, `championName`, `currentAugmentIds`, `currentBuildItemIds`, `hideLowPickrate`, `agentOperations`, `itemsCatalog`, `onSelectAugment`, `onSelectItem`

- **Data source**: `api/agent → requestStatistics` (global or personal scope)
- **Filters**: excludes components/starters/boots (`isLegendaryItem`, `isBootOrStarter`), min 3 games
- **Shape tabs**: All, 1A+1I, 1A+2I, 2A+1I, 2A+2I, 2A+3I
- **Sort**: WPA ↔ Win% ↔ Games
- **Search**: filter by name
- **Dedup**: max 2 entries per item/augment type
- **Each row**: item/augment icons (left), metrics (WPA, Win Rate, Games) (right)
- **Matched indicator**: green dot if item/augment already in current build
- **On select**: emits `onSelectItem({id, name, icon_url, price_total, item_class, description})` or `onSelectAugment({id, name, icon_url, rarity, description})`

### 4.4 `MayhemAugmentPicker.svelte` — In‑Game Augment Selection
**Props**: `championId`, `slots`, `fallbackUsed`, `onChange`

- 4 slots (levels 3, 7, 11, 15), each holds one augment or empty
- Loads from `api/champ_select → getMayhemAugments` (filtered by champ, excludes already‑picked)
- Searchable input when slot is open
- On pick/clear → emits new `augmentIds` via `onChange`
- Display: icon + name + WPA% · Xg badge

### 4.5 `LiveMayhemOverlay.svelte` — Webview Fallback Overlay
- Mounted when Tauri launches with `?window=overlay` arg
- Canvas‑based HUD: augment cards with tier borders (Silver/Gold/Prismatic), synergies, stats
- Synchronizes champion/augments/items with `MayhemChampSelectView.svelte` via `overlay:state-sync` Tauri event
- **Note**: primary in‑game overlay is the native Win32 process (`native_overlay.rs`); this webview overlay is a secondary UI

---

## 5. Overlay Bridge: Frontend → Native Win32 Overlay

### 5.1 `emitOverlayState` Payload (`MayhemChampSelectView.svelte:241-253`)
```typescript
emitOverlayState({
  source: "main_window",
  championId,
  championName,
  strategyId: "engine_v2",
  augmentIds,
  recommendedBoots: rec?.recommended_boots?.item_id,
  nextItemIds: rec?.next_items?.map(i => i.item_id) ?? [],
  completedBuild: rec?.completed_build?.map(i => (typeof i === "number" ? i : i.item_id)) ?? [],
  buildEngineRec: rec,
})
```

### 5.2 Tauri Command → Rust (`src-tauri/src/services/live_overlay.rs:314-446`)
`notify_overlay_state_from_frontend(&serde_json::Value)` extracts:
- `recommendedBoots`: tolerant field path (`recommendedBoots` or `recommended_boots`)
- `nextItemIds` / `next_item_ids` / `nextItems` / `next_item_ids`
- `completedBuild` / `completed_build` / `completed_build`
- `augmentIds` / `augment_ids`

### 5.3 Sprite Generation (`live_overlay.rs:153-229`)
`build_recommended_item_sprites(boots, next_items, completed_build)`:
1. Build list of unique item IDs: boots → next → completed (order: boot, next items, completed build)
2. Cap at 7 items total
3. For each item:
   - Resolve/download CommunityDragon PNG to `%LOCALAPPDATA%\lcu-companion\cache\cdragon\items\{id}.png`
   - Read bytes or fallback CDragon URL
4. Position: banner at bottom‑right of HUD, 36×36 icons, 8px gap, starting x = `(screen‑width‑340‑24)`, y = `18 + 42 + 28`
5. **Tier borders**: boot → amber `(245,158,11)`, next → emerald `(16,185,129)`, build → slate `(71,85,105)`

### 5.4 Synergy Row Computation (`live_overlay.rs:418-432`)
When item keys present AND champion + augments provided:
- `compute_overlay_synergy_rows(champion_id, augment_ids, owned_items, 3)` → top 3 synergy rows
- Each row has `item_id`, `icon_path` (cached on‑disk), `win_rate`, `wpa`, `games`
- Icons downloaded and cached same as item sprites

### 5.5 Native Overlay Render (`native_overlay.rs:1026-1269`)
Renders to target window buffer:
1. **Recommended Items Box**: semi‑transparent container (rgba 9,13,22/230) under HUD banner
   - Header: "RECOMMENDED ITEMS" text
   - Each sprite: rounded rect border (boot=amber, next=emerald, build=slate), PNG blitted 36×36
2. **Synergy Panel**: collapsible panel under items box
   - Header: "SYNERGY ITEMS" text
   - Each row: 22×22 icon badge + name + WPA% + Win Rate% + games
   - Tier borders same palette

### 5.6 Two Overlay Modes
| Mode | Mechanism | Renderer |
|---|---|---|
| **Native Win32** | Detached Tauri process (`lcu-overlay`), IPC via `OverlayIpcCommand::Render` | GDI `BitBlt` + `TextRenderer`, PNG sprites + tier borders |
| **Webview HTML5** | `LiveMayhemOverlay.svelte` mounted via `?window=overlay` | Canvas 2D draws rects, text, PNG images |

---

## 6. UI Completeness Verification

| Feature | Backend | Tauri API | Frontend Component | Overlay |
|---|---|---|---|---|
| **Item Build (next 2 items)** | `bitset_miner.rs` → `BuildRecommendationResult.next_items` | `api_compute_mayhem_build_recommendation` | `MayhemBuildEnginePanel` → Next Items grid | `build_recommended_item_sprites` → "next" border + PNG |
| **Item Build (completed 5‑item path)** | `bitset_miner.rs` → `completed_build: Vec<ItemRecommendation>` (5 legendaries) | same | `MayhemBuildEnginePanel` → grid-cols-5 with WR/lift/games | sprites positioned as "build" tier + PNG |
| **Item Build (recommended boots)** | `bitset_miner.rs` → `recommended_boots: Option<ItemRecommendation>` | same | `MayhemBuildEnginePanel` → single boot slot (cyan border) | sprite with amber border at banner start |
| **Augment Selection** | `bitset_miner.rs` active combos include augment_ids; `getMayhemAugments` returns augmented data | `api_compute_mayhem_build_recommendation`, `getMayhemAugments` | `MayhemAugmentPicker` (4 slots) + `MayhemChampSelectView` augment toggle | overlay augments via `offeredAugments` + tier borders |
| **Combo Synergies (active_combos)** | `bitset_miner.rs` → `active_combos: Vec<ActiveComboSynergy>` | same | `MayhemBuildEnginePanel` → horizontal chips + `MayhemCombosCard` (separate tab) | `native_overlay.rs` synergy panel + item icons |
| **Engine Stats** | `bitset_miner.rs` → `EnginePerformanceStats` | same | `MayhemBuildEnginePanel` → banner (time, matches, cache hit) | N/A (frontend only) |
| **Alternative Items** | `bitset_miner.rs` → `alternative_items: Vec<ItemRecommendation>` | same | `MayhemBuildEnginePanel` → "Alternative Options" section | not forwarded to native overlay (frontend only) |

**All layers verified**: item builds (boots + 5‑item path + next items + alternatives), augment selection (picker + active combos), combo synergies (engine combos + global combos), and native overlay rendering (sprites + banners + tier borders) all traverse the full data chain.

---

## 7. Flow Diagram (Top‑Level)

```
[LCU / Live Game State]
       │
       ▼
[Rust Build Engine Subprocess]
   (bitset_miner.rs / data_loader.rs / item_catalog.rs)
       │ produces BuildRecommendationResult
       ▼
[Tauri Command] api_compute_mayhem_build_recommendation
       │
       ▼
[Frontend] src/lib/api/champ_select.ts
       │
       ├─→ $effect recompute on champ/items/augments change
       ├─→ knownItemMap / knownAugmentMap derivation
       ├─→ MayhemBuildEnginePanel.svelte (3‑col grid: next/boots/build/alt/combos/stats)
       ├─→ MayhemBuildSimulator.svelte (6‑slot interactive simulator)
       ├─→ MayhemCombosCard.svelte (global ARAM combo stats, tabbed, search/sort)
       ├─→ MayhemAugmentPicker.svelte (4 in‑game augment slots)
       └─→ emitOverlayState({recommendedBoots, nextItemIds, completedBuild, buildEngineRec})
           │
           ▼
[Rust] live_overlay.rs notify_overlay_state_from_frontend
       │
       ├─→ build_recommended_item_sprites() → PNG assets + tier borders
       ├─→ compute_overlay_synergy_rows() → synergy item icons + labels
       └─→ refresh_native_overlay() → OverlayRenderPayload → native Win32 overlay window
```

---
*Document generated from codebase audit on 2026-10-31. All paths resolved against the current implementation.*