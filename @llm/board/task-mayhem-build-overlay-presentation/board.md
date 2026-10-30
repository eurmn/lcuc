# Task Board: task-mayhem-build-overlay-presentation

## Task Details
- **Task ID**: `task-mayhem-build-overlay-presentation`
- **Task Name**: Frontend Mayhem Build Engine Presentation & Native Overlay CDragon PNG Sprite Rendering
- **Status**: In Progress
- **Created**: 2026-10-29

---

## Context & User Goal
The backend modularized the Mayhem Build Engine into an independent subprocess (`lcu-build-engine`) exposed via `api_compute_mayhem_build_recommendation`. Now the front-end must consume this output (`BuildRecommendationResult`: boots, next 2 items, 6-item completed build, and active combo synergies), display it in the Champ Select & Live Game views with CommunityDragon item icons, and forward the data to the proprietary detached Win32 overlay (`lcu-overlay`) to render the recommended item sprites via direct PNG blitting.

---

## Architecture Plan & Contract

### 1. Frontend API & Types (`src/lib/types/champ_select.ts` & `src/lib/api/champ_select.ts`)
- TypeScript models:
  - `ActiveComboSynergy` (combo_type, item_ids, augment_ids, label, win_rate, wpa, sample_size)
  - `ItemRecommendation` (item_id, win_rate, lift, sample_size, is_boot, reason)
  - `BuildRecommendationResult` (champion_id, active_combos, next_items, recommended_boots, completed_build, stats)
- API function `computeMayhemBuildRecommendation(championId: number, ownedItems: number[], ownedAugments: string[])` invoking `api_compute_mayhem_build_recommendation`.

### 2. Frontend Live Game & Overlay Bridge (`src/lib/components/history/MayhemChampSelectView.svelte`)
- When live game state updates (or simulation changes):
  - Request dynamic build recommendations using `computeMayhemBuildRecommendation`.
  - Render dynamic build advice card (Boots, Next 2 Items, Completed Build, Combos) with CommunityDragon icon URLs (`https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/{id}.png`).
  - Forward recommended items and build advice payload to `emitOverlayState`.

### 3. Backend Overlay Sprite Bridge (`src-tauri/src/services/live_overlay.rs`)
- In `notify_overlay_state_from_frontend`:
  - Extract recommended item IDs (boots, next items, completed build).
  - Resolve or download local cached PNG paths using `crate::services::asset_cache::get_asset_cache_dir()` and `get_cached_asset_data_uri`.
  - Generate `OverlayPngSprite` entries placed at calibrated screen coordinates.
- In `refresh_native_overlay`:
  - Pass the populated `sprites` list into `OverlayRenderPayload`.

### 4. Proprietary Native Rust Overlay Renderer (`src-tauri/src/services/native_overlay.rs`)
- Render recommendation bar on top-right below the HUD banner:
  - Display "NEXT ITEMS" / "BUILD PATH" text.
  - Blit item PNG sprites with rounded borders or item boxes.
  - Render boot recommendation if present.

---

## Strict File Ownership
- **Worker 1 (Frontend Svelte/TS Specialist)**:
  - `src/lib/types/champ_select.ts`
  - `src/lib/api/champ_select.ts`
  - `src/lib/components/history/MayhemChampSelectView.svelte`
- **Worker 2 (Rust Overlay & Backend Specialist)**:
  - `src-tauri/src/services/live_overlay.rs`
  - `src-tauri/src/services/native_overlay.rs`

---

## Log Entries
### [@orchestrator] 2026-10-29
- Initialized board and dispatched Wave 2 workers with disjoint file ownership.

### [@worker-native-overlay] 2026-10-29 18:30
- Extended `live_overlay.rs`:
  - Added global `OVERLAY_RECOMMENDED_SPRITES: Mutex<Vec<OverlayPngSprite>>`.
  - Added `clear_overlay_recommended_sprites()` and wired `refresh_native_overlay()` to forward `OVERLAY_RECOMMENDED_SPRITES` inside `OverlayRenderPayload.sprites`.
  - Extended `notify_overlay_state_from_frontend(&serde_json::Value)` to extract `recommendedBoots`, `nextItemIds`, and `completedBuild` across top-level and nested advising payloads without blocking locks.
  - Implemented async helper `build_recommended_item_sprites`: checks/downloads CommunityDragon 2D item icons into `%LOCALAPPDATA%/lcu-companion/cache/cdragon/items/{id}.png` and positions 36x36 item tiles with 8px spacing below the HUD banner.
- Extended `native_overlay.rs`:
  - Modularized `render_to_buffer(&mut self, buf: &mut [u8], layout: &OverlayAnchorLayout, payload: &OverlayRenderPayload)`.
  - Implemented sleek semi-transparent container box and "RECOMMENDED ITEMS" header for recommended item sprites below the HUD banner.
  - Added role-based item borders (Amber for Boots, Emerald for Next Items, Slate for Target Build) and blitted sprites using `blit_rgba_sprite`.
  - Added unit test `test_recommended_items_sprites_blitting` validating background box and sprite blitting onto the BGRA buffer.

### [@worker-frontend-svelte] 2026-10-29 18:35
- Implemented Typed Rust Build Engine Models in `src/lib/types/champ_select.ts`:
  - `ActiveComboSynergy`, `ItemRecommendation`, `EnginePerformanceStats`, and `BuildRecommendationResult`.
- Added API Invoker in `src/lib/api/champ_select.ts`:
  - `computeMayhemBuildRecommendation(championId, ownedItems, ownedAugments)` calling `api_compute_mayhem_build_recommendation`.
  - Cleaned up duplicate type imports and unused declarations.
- Updated `src/lib/components/history/MayhemChampSelectView.svelte`:
  - Added reactive state `buildEngineRec = $state<BuildRecommendationResult | null>(null)`.
  - Added derived mappings `knownItemMap` and `knownAugmentMap` for fast entity lookups.
  - Implemented `refreshBuildRecommendation` and Svelte 5 `$effect` to recompute recommendations reactively when champion, simulated augments, or build item slots change.
  - Extended `refreshLiveGameState` to invoke `computeMayhemBuildRecommendation` with live inventory item IDs on each poll.
  - Enriched `emitOverlayState` calls with `recommendedBoots`, `nextItemIds`, and `completedBuild`.
  - Built real-time "Mayhem Build Engine Advice" card rendering:
    - Recommended Boots with CDragon item icon, Win Rate, and Lift badge.
    - Top 2 Next Priority Items with CDragon item icons, Win Rate, and Lift badges.
    - Full 6-Item Completed Build sequence with CDragon item icons and rank indicators.
    - Active Combos & Synergy Pairs with item & augment icons, combo labels, Win Rate, and WPA badges.
    - Engine performance statistics badge (compute time, match sample count, cache status).
- Verified via `type_check` and `svelte-autofixer`: 0 errors.
