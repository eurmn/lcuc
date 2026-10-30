# Mayhem Overlay Performance and OCR Optimization Walkthrough

## 1. Problem

- Fullscreen transparent WebView2 window caused ~50% in-game FPS drop in League ARAM Mayhem:
  - Loss DWM DirectFlip compositing path.
  - Costly CSS `backdrop-blur-md` filters and heavy drop-shadows across fullscreen layer.
  - Active 40ms cursor polling thread toggling `WS_EX_TRANSPARENT` via `SetWindowLongPtrW`.
- Augment card OCR recognition failed:
  - Bounding box captured full card heights including category tags and descriptions.
  - Joined lines column diluted string similarity scores below match threshold.
- Augment card appearance timing:
  - Cards appear rounds champion levels 3, 7, 11, and 15 upon game start or respawn.

## 2. Solution Implemented

### Performance & Window Management
- Permanently click-through window styles:
```rust
WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
```
- Removed `ensure_overlay_cursor_thread`, `OVERLAY_INTERACTIVE_RECTS`, and mouse hit-testing loops.
- Window hidden (`SW_HIDE`) when no cards active.
- Sized strictly to banner strip augment cards instead of covering fullscreen:
```rust
x: rect.x + 0.20 * w
y: rect.y + 0.11 * h
w: 0.60 * w
h: 0.075 * h
```
- Shown via `ShowWindow(SW_SHOWNOACTIVATE)` without taking focus or dropping DirectFlip on game client.

### Hotkey Scanning & Hide Controls (`live_overlay.rs` & `live_game.rs`)
- Replaced automatic milestone scanning and interactive "Scan Cards" button with global hotkeys:
  - `F8` (`VK_F8 = 0x77`): Scan / rescan all 3 cards after refresh or reroll.
  - `F7` (`VK_F7 = 0x76`): Hide cards overlay immediately.
- Hotkeys read via `GetAsyncKeyState` with 75ms polling in `live_game.rs`.
- Avoids default League of Legends keybindings (`F1`-`F5`, `F9`, `F12`).

### Frontend & HTML5 Canvas Renderer (`LiveMayhemOverlay.svelte`)
- Removed `backdrop-blur-md`, heavy shadows, and DOM measurement polling loops.
- Replaced DOM card rendering with lightweight HTML5 Canvas 2D renderer (`cardCanvas`).
- Draws all 3 cards' Win Rate %, WPA, sample size, and Top Combos & Synergies on a single GPU layer with zero `backdrop-blur` overhead.
- Subscribes to `"overlay:cards-updated"`.

### Tight OCR Detection & Combo Enrichment (`live_overlay.rs`)
- Tightened crop band to augment title headers:
```rust
rx = 0.21, ry = 0.385, rw = 0.58, rh = 0.065
```
- Groups extracted OCR lines into 3 distinct columns based on center X coordinate.
- Strips category tag words: `damage`, `speed`, `utility`, `resilience`, `quest`.
- Matches against augment catalog with similarity threshold >= 0.60; requires >= 2 matches to confirm card visibility.
- Enriches `OverlayAugmentCard` in `enrich_overlay_augment_cards`:
  - Bayesian WPA and WR.
  - `combos: Vec<OverlayComboEntry>` containing co-occurring augments and item pairs with Win Rate, WPA, and games.
