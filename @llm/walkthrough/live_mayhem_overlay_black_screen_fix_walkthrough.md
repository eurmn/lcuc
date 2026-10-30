# Live Mayhem Overlay Black Screen Fix Walkthrough

## 1. Problem Description
Overlay created black screen covering League of Legends client/game window. Only League mouse cursor rendered on top. Overlay contents not transparent; game underneath completely occluded.

## 2. Root Causes Identified
- **WebView2 Child Window Ex-Styles Corruption**: `enum_child_proc` in `src-tauri/src/services/live_overlay.rs` traversed child HWNDs (`Chrome_WidgetWin_0`, `Intermediate D3D Window`) and applied `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE`. Broke Chromium DirectComposition swapchain; turned surface into opaque uninitialized black GDI buffer.
- **DWM DirectFlip Occlusion**: Sizing overlay window to exact dimensions (`rect.width x rect.height`) matched fullscreen/borderless DirectX game swapchain. Triggered Windows Desktop Window Manager (DWM) DirectFlip optimization; treated overlay as exclusive fullscreen plane, occluding game DXGI swapchain.
- **Unthrottled DWM Composition Churn**: 25Hz cursor check thread called `SetWindowPos` and `ShowWindow` on tick unconditionally without state diffing. Caused continuous DWM composition disruption and flicker.
- **Static Opaque CSS Background**: `index.html` hardcoded `bg-gray-900` class on `<body>` and `<div id="app">`. Rendered solid dark gray background on load.
- **Missing Win32 Transparent Background on Window Init**: Overlay window lacked explicit `Some(tauri::window::Color(0, 0, 0, 0))` background configuration during Tauri startup.

## 3. Changes Applied

### `src-tauri/src/services/live_overlay.rs`
- Deleted `enum_child_proc`. Configured top-level HWND ex-styles only (`WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE`).
- Added `overlay_safe_height` (`(rect.height - 1).max(1)`) to avoid DWM DirectFlip fullscreen blackouts.
- Cached position, visibility, and cursor state in 25Hz loop to avoid redundant `SetWindowPos` and `ShowWindow` calls.
- Fixed interactive hitbox tuple unpacking `&(rx, ry, rw, rh)` for cursor collision detection.

### `src-tauri/src/lib.rs`
- Explicitly configured overlay window zero-alpha background `Some(tauri::window::Color(0, 0, 0, 0))` and applied Win32 overlay styles in `setup`.

### `index.html`
- Removed static `bg-gray-900` from `<body>` and `<div id="app">`.
- Added inline `<script>` in `<head>` setting `overlay-window` class and transparent background before first paint.

### `src/main.ts`
- Dynamically applies `bg-gray-900` only when `!isOverlay`. Keeps overlay transparent.

## 4. Verification Results
- `cargo check` verified clean (0 errors, 0 warnings).

## 5. Verification & Testing Advice
1. **Window Transparency Verification**:
   - Launch app with overlay enabled. Ensure desktop/background windows show through overlay bounds when no cards visible.
   - Verify HTML elements (e.g. Mayhem augment recommendations, HUD badges) render correct alpha blending.
2. **Game Ingestion & Occlusion Test**:
   - Start League custom/practice game in Borderless and Fullscreen modes.
   - Confirm game DXGI swapchain renders behind overlay without black screen or frame dropping.
   - Verify mouse click pass-through when cursor outside interactive overlay regions (`WS_EX_TRANSPARENT`).
3. **DWM Performance Check**:
   - Monitor `dwm.exe` CPU/GPU usage via Task Manager during match. Ensure 25Hz loop no longer causes high composition load from redundant `SetWindowPos` calls.
