# Sub-Board: Dual-Signal OCR Card Matcher (Name + Description)

## Task ID
`task-ocr-dual-signal-matcher`

## Problem Context
Augment cards scanned via OCR in League of Legends ARAM Mayhem have suffered from false positives (e.g. matching "Ice Cold" or "Infinite Recursion" on "Blade Waltz" or "Recursion") and missed detections.
The user directive is:
1. Scan both the **Title** and **Description** of each card.
2. Formulate a dual-signal matching algorithm where both the title and descriptive keywords must corroborate the match. Description matching must allow higher error tolerance (due to icons, stylized text, dynamic values).
3. Verify against all 8 sample images in `@llm/cache/augment-selection-examples/` (`Screen15.png`, `Screen68.png`, `Screen70.png`, `Screen71.png`, `Screen72.png`, `Screen74.png`, `Screen75.png`, `Screen76.png`).
4. The task is only complete when all cards across all sample images are accurately and unambiguously identified.

## Architecture & Algorithm
- **OCR Engine**: Windows Media OCR (`windows::Media::Ocr::OcrEngine`) on native Win32.
- **Card Slicing**:
  - Horizontal bounds: Card 1 (Left ~15% - 40%), Card 2 (Center ~38% - 62%), Card 3 (Right ~60% - 85%).
  - Vertical bounds: Title band (~28% - 45%), Description band (~45% - 75%).
- **Dual-Signal Corroboration**:
  - `TitleScore`: Exact string match, token containment, or Levenshtein distance on normalized title words.
  - `DescriptionScore`: Bag-of-words / keyword presence from augment description text (stripped of HTML tags, numbers, percent signs, and generic stop words).
  - Both signals must agree. A strong title match with 0 description match is penalized/rejected, and vice-versa.

## Progress Log
### [@orchestrator] 2026-10-30
- Initialized sub-board and task.
- Target images confirmed in `@llm/cache/augment-selection-examples/`.
- Implemented `DualSignalMatcher` in `src-tauri/src/services/dual_matcher.rs` with HTML/placeholder stripping, word Levenshtein similarity, multi-word title ratio penalties, and description keyword matching.
- Integrated multi-pass luminance binarization (`recognize_text_lines_multi_pass` at `130.0` and `150.0` luminance thresholds + raw fallback) and `DualSignalMatcher` into `match_ocr_lines_to_augments`, `scan_title_band_from_image`, and `scan_in_game_augment_cards` in `src-tauri/src/services/live_overlay.rs`.
- Tuned 3-card crop region (`x: 15%..85%`, `y: 18%..72%`), horizontal column boundaries (`0.36` and `0.65`), and full catalog page size (`1000`).
