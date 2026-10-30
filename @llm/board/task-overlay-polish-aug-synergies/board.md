# Task Board: Overlay Recommended Sprites, OCR Reliability, and Augment Synergies

- **Task ID**: `task-overlay-polish-aug-synergies`
- **Goal**:
  1. Fix recommended item sprites not showing in native overlay (send image bytes directly via IPC and fix coordinate clamping).
  2. Overhaul augment card overlay synergies to show 1-2 commonly bought legendary items (empirical bitset co-occurrence, filter low pick rate / components).
  3. Improve OCR card scanning band, allow 1+ detected cards, and remove silent discard.
  4. Implement on-screen toast/error feedback in native overlay and frontend when scan succeeds/fails.

### Work Log
- 2026-10-30: Task board initialized.
- 2026-10-30: Recommended item sprites now encode and send raw PNG bytes (`png_base64_or_bytes`) directly over IPC, bypassing cross-process file-path resolution issues.
- 2026-10-30: Replaced legacy augment combo loops in `history.rs` (`enrich_overlay_augment_cards`) with empirical bitset co-occurrence mining (`find_top_legendary_items_for_augment`). Restricts synergizing items strictly to legendaries, filters components/boots, and enforces pick rate / sample size gates.
- 2026-10-30: OCR scanning band widened (from 6.5% to 12% height), allowing 1+ matched cards, returning clear descriptive errors if no cards or no text are detected.
- 2026-10-30: Added on-screen toast banners in `native_overlay.rs` (drawn via GDI font renderer) and frontend `MayhemChampSelectView.svelte` & `LiveMayhemOverlay.svelte` displaying scanning progress, card count, or scan errors.
- 2026-10-30: Verified clean TypeScript type check and Rust unit tests (native overlay + build engine).

