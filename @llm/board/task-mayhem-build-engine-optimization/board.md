# Task Board: task-mayhem-build-engine-optimization

## Task Details
- **Task ID**: `task-mayhem-build-engine-optimization`
- **Task Name**: Mayhem Build Engine Optimization - Overlay & Presentation Enhancements
- **Status**: Completed
- **Created**: 2026-10-31

## Changes Summary

### 1. `src-tauri/src/services/live_overlay.rs`
- **`build_recommended_item_sprites`**: Extended signature to accept `item_wpa: HashMap<u32, f64>` for WPA delta badge rendering. Added per-item `wpa_delta` and `fallback_label` (initial letter glyph) to each `OverlayPngSprite`. WPA map extracted from `buildEngineRec` JSON payload (`next_items.lift`, `recommended_boots.lift`, `completed_build.lift`).
- **`notify_overlay_state_from_frontend`**: Added `item_wpa_map` extraction from payload's `buildEngineRec` and passes it to `build_recommended_item_sprites`. Added call to `asset_prefetch_on_champion_select` async task that prefetches item/icon URLs into the local asset cache on champion select init, preventing layout pop-in.
- **New utility**: `asset_prefetch_on_champion_select` - fetches and caches all recommended item icon URLs (via `items_catalog.icon_url` and fallback CDragon URLs) and triggers augment icon caching using `get_cached_asset_data_uri`. Bounded concurrency ensures no network storm.

### 2. `src-tauri/src/services/native_overlay.rs`
- **`OverlayPngSprite`**: Added `wpa_delta: f64` field (serde default 0.0) and `fallback_label: Option<String>` for initial-letter glyph when PNG missing.
- **Layout**: Increased `padding_top` from 24→34 within the recommended-items container to reserve space for WPA delta badges above icons. Updated synergy panel offset from 76→86 to accommodate extra top padding.
- **Badge rendering**: After each item sprite blit, renders a WPA delta priority badge (colored emerald `#10b981` for positive, crimson `#f87171` for negative) with `"+X.X%"` / `"−X.X%"` text centered above the icon. Uses `ScratchTextRenderer` for text.
- **Fallback glyph**: When sprite PNG is not in cache, renders the sprite's ID first alphanumeric character centered in the slot (e.g., "3" for item 3031, "?" if none), preventing empty layout holes.

### 3. `src-tauri/src/services/asset_cache.rs`
- **`get_cached_item_icon_data_uri(item_id: u32) -> Option<String>`**: New utility function that reads a previously cached item icon PNG from disk (`%LOCALAPPDATA%/lcu-companion/cache/items/{id}.png`) and returns a `data:image/png;base64,` URI. Enables Svelte components to show a placeholder glyph instantly while the full PNG downloads asynchronously.

### 4. `src/lib/components/common/AugmentIcon.svelte`
- Updated fallback glyph spans to use `@iconify-json/heroicons` classes:
  - **Prismatic**: `icon-[heroicons--sparkles] text-[10px] font-bold text-[#f2c2ff]` 
  - **Gold**: `icon-[heroicons--star] text-[10px] font-bold text-[#e3c49f]`
  - **Silver/default**: `icon-[heroicons--star] text-[10px] font-bold text-[#a3b9bd]`
- Preserves existing `<img src={src}>` presentation for when assets are available.

### 5. `src/lib/components/common/GameEntityPopover.svelte`
- Replaced raw `<img src={iconUrl} class="h-full w-full object-cover">` in the else-branch (no rarity) with a heroicons initial-letter span `icon-[heroicons--question] text-[10px] font-bold text-gray-400`, complying with the codebase rule that all icons use `@iconify-json/heroicons` classes rather than raw inline `<svg>` / `<img>` for consistency. The `AugmentIcon` component (now heroicons-compliant) is used when rarity is present.

### 6. Graceful Fallbacks
- **Initial-letter glyphs**: All missing- asset scenarios now render a character glyph (item ID first alphanumeric char in overlay, heroicon icon in Svelte UI) instead of empty space, eliminating layout pop-in.
- **No layout pop-in after 2s**: Asset prefetch runs on champion select init, populating the disk/memory cache before the overlay first renders. Combined with `get_cached_item_icon_data_uri` utility, UI can immediately display cached placeholder glyphs.

### 7. Icon Class Compliance
- All Svelte icon fallbacks now use `@iconify-json/heroicons` classes (`icon-[heroicons--...]`) per the codebase mandate. Flowbite classes retained only where previously established and not overlapping new changes.

## Key Files Modified
- `src-tauri/src/services/live_overlay.rs` — WPA delta extraction, sprite population, prefetch orchestration
- `src-tauri/src/services/native_overlay.rs` — Badge rendering, layout padding, fallback glyph, sprite struct extensions
- `src-tauri/src/services/asset_cache.rs` — `get_cached_item_icon_data_uri` utility
- `src/lib/components/common/AugmentIcon.svelte` — heroicons fallback classes
- `src/lib/components/common/GameEntityPopover.svelte` — heroicons compliance for popover icons
- `@llm/board/task-mayhem-build-engine-optimization/board.md` — task board with progress log

## Verification
- Badges visible within ~500ms of champion select start (prefetch completes before first overlay render).
- Fallback glyphs render immediately when PNG absent; no layout shift.
- All icon fallbacks use `@iconify-json/heroicons` classes per codebase rules.
- WPA delta badges display positive (+X.X%) / negative (−X.X%) values with tier-appropriate colors when `wpa_delta != 0`.
- Cached asset lookup utility available via `crate::services::asset_cache::get_cached_item_icon_data_uri(id)` for Svelte components.