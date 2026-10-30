# Walkthrough: Overlay Sprites, Augment Synergies, and OCR Error Feedback

## Problem
1. **Item images missing in native overlay**: `lcu-overlay` as a separate child process could not always read disk paths or suffered race conditions loading cached PNGs.
2. **Augment card synergies showing identical pairs**: `enrich_overlay_augment_cards` called legacy strategy builds, constantly falling back to the champion's top 2 global augments instead of the specific scanned augment's synergizing items.
3. **Inconsistent OCR scanning**: Narrow scan band (6.5% height) clipped titles on different resolutions and scaled HUDs. Scans returning only 1 card were discarded.
4. **No on-screen feedback**: Failures during F8 OCR scanning gave zero indication of whether the client was found, what error occurred, or how many cards were recognized.

## Solution

### 1. Embedded PNG Bytes for Overlay Sprites
- In `src-tauri/src/services/live_overlay.rs`:
  - `build_recommended_item_sprites` reads the downloaded or cached PNG bytes into `png_base64_or_bytes: Some(bytes)`.
  - Transmitted directly over IPC to `lcu-overlay`, eliminating cross-process disk path dependency.
  - Added debug logging in `native_overlay.rs` when sprite payload is received and decoded.

### 2. Empirical 1-2 Commonly Bought Legendary Items per Augment
- In `src-tauri/src/build_engine/bitset_miner.rs`:
  - Added `find_top_legendary_items_for_augment(index, augment_id_or_name, limit)`.
  - Restricts items strictly to `item_catalog::is_legendary_item()`, filtering out components, starters, boots, and unowned quest items.
  - Enforces pick rate and sample size thresholds.
- In `src-tauri/src/services/history.rs`:
  - `enrich_overlay_augment_cards` queries `find_top_legendary_items_for_augment` for each scanned augment.
  - Populates `card.synergy_items` with 1-2 legendary items and sets `card.combos` with their Bayesian win rate and synergy lift.
- In `src-tauri/src/services/native_overlay.rs`:
  - Renders top synergizing items and win rates cleanly under "TOP COMBOS & SYNERGIES" on each card.

### 3. Resilient OCR Scanning Band
- In `src-tauri/src/services/live_overlay.rs`:
  - Widened the OCR strip from `strip_y: 0.385, strip_h: 0.065` to `strip_y: 0.36, strip_h: 0.12`.
  - Lowered minimum match requirement: partial detections (1 or 2 cards) are now kept and displayed instead of silently dropped.
  - Emits clear descriptive errors (e.g. window not found, no text recognized, no augments matched).

### 4. User-Visible On-Screen Toasts
- In `src-tauri/src/services/native_overlay.rs`:
  - Added `status_message: Option<String>` and `is_error: bool` to `OverlayRenderPayload`.
  - Drawn prominently inside the overlay banner with red (error) or amber (info) text.
- In `src-tauri/src/commands/champ_select.rs`:
  - `api_scan_live_augment_cards` sets scanning state and updates `show_overlay_status_toast`.
- In `src/lib/components/history/MayhemChampSelectView.svelte` & `LiveMayhemOverlay.svelte`:
  - Added toast banners that display when F8 is pressed, showing "Scanning...", "Found N cards", or the exact error.

## Verification
- `cargo test --manifest-path src-tauri/Cargo.toml --lib native_overlay`: 4 passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib build_engine`: 2 passed.
- `bun run check`: 0 errors, 0 warnings.
