# Walkthrough: Mayhem Build Engine Presentation & Native Overlay CDragon PNG Sprite Rendering

## 1. Feature Summary
Wired the modular ARAM Mayhem Build Engine into the Svelte 5 frontend and the detached native Win32 Rust overlay (`lcu-overlay`).
- The frontend consumes `BuildRecommendationResult` from the background engine process via `computeMayhemBuildRecommendation`.
- Renders dynamic build advice cards in `MayhemChampSelectView.svelte` showing recommended boots, next 2 priority items, completed 6-item builds, and active combo synergies with CommunityDragon item icons.
- Passes recommended item sets to `emitOverlayState`.
- The native Rust overlay supervisor (`live_overlay.rs`) caches CommunityDragon item PNGs to disk and positions `OverlayPngSprite` tiles.
- The native layered window renderer (`native_overlay.rs`) draws a semi-transparent HUD box with "RECOMMENDED ITEMS" text, role-colored borders, and direct premultiplied BGRA PNG sprite blitting.

---

## 2. Changes

### A. Frontend Data Models & IPC Adapter
- **`src/lib/types/champ_select.ts`**:
  - Added `ActiveComboSynergy`, `ItemRecommendation`, `EnginePerformanceStats`, and `BuildRecommendationResult`.
- **`src/lib/api/champ_select.ts`**:
  - Added `computeMayhemBuildRecommendation(championId, ownedItems, ownedAugments)` calling Tauri command `api_compute_mayhem_build_recommendation`.

### B. Frontend Champ Select & Live Advice UI
- **`src/lib/components/history/MayhemChampSelectView.svelte`**:
  - Added reactive state `buildEngineRec` and entity maps `knownItemMap` / `knownAugmentMap`.
  - Added reactive triggers to recompute recommendations when active champion, simulated augments, or build item slots change.
  - Added `refreshLiveGameState` hook querying the build engine with live inventory item IDs on each poll.
  - Forwarded `recommendedBoots`, `nextItemIds`, `completedBuild`, and recommendation payloads into `emitOverlayState`.
  - Added real-time "Mayhem Build Engine Advice" card rendering:
    - Boots recommendation with CommunityDragon 2D item icon, win rate, and lift badges.
    - Top 2 next priority items with CommunityDragon 2D item icons, win rates, and lift badges.
    - Completed 6-item build sequence with slot numbers and icons.
    - Active combo synergies (item-item and item-augment) with badges.
    - Performance stats pill (execution latency, sample size, cache hit status).

### C. Backend Asset Caching & Overlay Sprite Synchronization
- **`src-tauri/src/services/live_overlay.rs`**:
  - Added `OVERLAY_RECOMMENDED_SPRITES: Mutex<Vec<OverlayPngSprite>>` and `clear_overlay_recommended_sprites()`.
  - Wired `refresh_native_overlay()` to inject `OVERLAY_RECOMMENDED_SPRITES` into `OverlayRenderPayload.sprites`.
  - Extended `notify_overlay_state_from_frontend` to extract `recommendedBoots`, `nextItemIds`, and `completedBuild`.
  - Implemented async asset cacher downloading item icons from `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/{id}.png` into `%LOCALAPPDATA%/lcu-companion/cache/cdragon/items/{id}.png`.
  - Positioned 36x36 item tiles with 8px spacing directly below the HUD banner.

### D. Detached Win32 Native Overlay Rendering
- **`src-tauri/src/services/native_overlay.rs`**:
  - Modularized `render_to_buffer` to draw a sleek semi-transparent box with "RECOMMENDED ITEMS" header below the HUD banner.
  - Rendered role-based item borders (Amber for Boots, Emerald for Next Items, Slate for Completed Build) and blitted PNG sprites via `blit_rgba_sprite`.
  - Added unit test `test_recommended_items_sprites_blitting`.

---

## 3. Verification
- Frontend: TypeScript type checking and Svelte 5 rune validation verified with 0 errors.
- Backend: Unit test `test_recommended_items_sprites_blitting` verified sprite blitting and layout.
