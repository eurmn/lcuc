# OCR Algorithm Findings Walkthrough

## Problem

Augment card OCR recognition failed in live ARAM Mayhem matches. Bounding boxes captured full card heights including category tags and descriptions, and joined lines diluted string similarity scores below the match threshold.

## Findings

### 1. Multi-pass OCR replaces single-pass

`recognize_text_lines_multi_pass` supersedes `recognize_text_lines` in `scan_title_band_from_image` and `scan_in_game_augment_cards`. Single-pass Tesseract output was incomplete on low-contrast in-game text; multiple passes at different preprocessing settings recover missed lines.

### 2. Crop region tightened

The OCR crop now targets the 3-card title band instead of full card heights:

- `scan_title_band_from_image`: crops full-screen images to x: 15%..85%, y: 18%..72% before OCR.
- `scan_in_game_augment_cards`: strip crop changed from x: 21%..79%, y: 36%..48% to x: 15%..85%, y: 18%..72%, covering titles and descriptions while excluding HUD/top-bar.

### 3. Column bucketing by center X

OCR lines are grouped into 3 columns based on relative X coordinate:

- `rx < 0.36` → column 0
- `rx < 0.65` → column 1
- otherwise → column 2

Thresholds adjusted from thirds (0.333 / 0.666) to 0.36 / 0.65 to better align with actual card positions.

### 4. Dual-signal matching replaces line-by-line similarity

`match_ocr_lines_to_augments` now uses `DualSignalMatcher` instead of per-line `text_similarity_score`. Each column is scored as a whole using:

- Joined column text normalized via `normalize_token`
- Per-word token list with category badge noise filtered out
- Candidate score combines title and description signals

Similarity threshold lowered from 0.60 to 0.55. Requires >= 2 matches to confirm card visibility.

### 5. Category badge noise stripping

`is_category_badge_noise` filters tag words: `damage`, `speed`, `utility`, `resilience`, `quest`. These appear on augment cards but are not part of augment names and caused false matches.

### 6. Augment catalog page size increased

`get_mayhem_augments` page_size raised from 300 to 1000 to ensure the full augment catalog is available for matching.

### 7. Tests added

- `test_levenshtein_and_fuzzy_similarity`: verifies `levenshtein_distance` and `fuzzy_string_similarity` on typo-tolerant pairs.
- `test_is_category_badge_noise`: verifies category tag filtering.

## Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --lib live_overlay`: OCR unit tests pass.
- `cargo build --manifest-path src-tauri/Cargo.toml --bins`: Finished clean dev profile.
