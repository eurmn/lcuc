# Mayhem Overlay Performance and OCR Optimization Walkthrough

## 1. Problem

- Fullscreen transparent WebView2 window caused ~50% in-game FPS drop in League ARAM Mayhem:
  - Loss of DWM DirectFlip compositing path.
  - Costly CSS `backdrop-blur-md` filters and heavy drop-shadows across fullscreen layer.
  - Active 40ms cursor polling thread toggling `WS_EX_TRANSPARENT` via `SetWindowLongPtrW`.
- Augment card OCR recognition failed:
  - Bounding box captured full card heights including category tags and descriptions.
  - Joined lines per column diluted string similarity scores below match threshold.
- Augment card appearance timing:
  - Cards appear during selection rounds at champion levels 3, 7, 11, and 15 upon game start or respawn.

## 2. Solution Implemented

### Performance & Window Management
- Permanently click-through window styles:
  ```rust
  WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
  ```
- Removed `ensure_overlay_cursor_thread`, `OVERLAY_INTERACTIVE_RECTS`, and mouse hit-testing loops.
- Window hidden (`SW_HIDE`) when no cards are active.
- Sized strictly to a top banner strip above augment cards instead of covering fullscreen:
  ```rust
  x: rect.x + 0.20 * w
  y: rect.y + 0.11 * h
  w: 0.60 * w
  h: 0.075 * h
  ```
- Shown via `ShowWindow(SW_SHOWNOACTIVATE)` without taking focus or dropping DirectFlip on game client.

### Frontend (`LiveMayhemOverlay.svelte`)
- Removed `backdrop-blur-md`, heavy shadows, and DOM measurement polling loops.
- Replaced with lightweight solid background banner subscribing to `"overlay:cards-updated"`.
- Displays `#1 Pick` rank badge, WPA, WR, and top 2 synergizing item icons per card slot.

### Tight OCR Detection (`live_overlay.rs`)
- Tightened crop band to augment title headers:
  ```rust
  rx = 0.21, ry = 0.385, rw = 0.58, rh = 0.065
  ```
- Groups extracted OCR lines into 3 distinct columns based on center X coordinate.
- Strips category tag words: `damage`, `speed`, `utility`, `resilience`, `quest`.
- Matches against augment catalog with similarity threshold >= 0.60; requires >= 2 matches to confirm card visibility.
- Enriches cards via `enrich_overlay_augment_cards` with Bayesian WPA and top 2 synergizing items from adaptive build strategy.

### Auto-Scan State Machine (`live_game.rs`)
- Tracks player status from Riot Live Client Data API:
  - Parses `is_dead` and `respawn_timer`.
- Arms milestone scans at levels 3, 7, 11, and 15 (game start and dead -> alive transitions).
- Dispatches screen scan, renders banner when cards match, and calls `SW_HIDE` on selection round close.
