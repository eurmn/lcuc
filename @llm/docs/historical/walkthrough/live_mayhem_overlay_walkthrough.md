# Live Mayhem In-Game Overlay Walkthrough

## 1. Vanguard-Safe External Window Architecture

Zero hook, zero injection, zero memory read. Riot Vanguard compliant.

- **Window Type**: Borderless, transparent, topmost secondary Tauri `WebviewWindow` (`"overlay"`, entry `/?window=overlay`).
- **Target Tracking**: Win32 `FindWindowW(..., "League of Legends (TM) Client")` locates game hwnd.
- **Alignment**: Polls client position via `GetClientRect` + `ClientToScreen`. Syncs overlay position and size to match game client area exactly.
- **Click Pass-Through**: Default state sets OS-level mouse ignore via recursive child HWND style manipulation. Overlay ignores mouse events over transparent regions. Only interactive UI elements (pills, cards, drawer buttons) toggle pointer events to receive clicks.

## 2. WebView2 DirectComposition Transparency & Click-Through Subsystem

Ensures click-through without breaking Windows transparency rendering or stealing focus from League client.

- **WebView2 DirectComposition Transparency**:
  - `WEBVIEW2_DEFAULT_BACKGROUND_COLOR=0` environment variable set in `src-tauri/src/lib.rs` before initializing WebView2. Forces DirectComposition transparent swapchain.
  - Removes default opaque `bg-gray-900` background class from `document.body` and `#app` in `src/main.ts` when detecting `window=overlay`.
  - `.overlay-window` in `src/styles.css` applies transparent background, ensuring clean alpha pass-through.
- **Recursive Win32 Child HWND Click-Through**:
  - `src-tauri/src/services/live_overlay.rs` uses Win32 `EnumChildWindows` traversing all nested WebView2 HWND children (`Intermediate D3D Window`, `Chrome_WidgetWin_0`, etc.).
  - Recursively applies `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE` across top-level window and all child HWNDs to prevent WebView2 native window from swallowing mouse events.
  - Uses `SW_SHOWNOACTIVATE` and `SetWindowPos` with `SWP_NOACTIVATE` so overlay displays without stealing focus from active game window.
- **Dedicated 40ms Rust Cursor Hit-Test Thread**:
  - Background loop polls cursor position every 40ms via Win32 `GetCursorPos`.
  - Compares cursor coordinates against physical bounding rects of DOM elements tagged with `[data-overlay-interactive="true"]`.
  - Dynamically toggles `WS_EX_TRANSPARENT` state on window hierarchy only when cursor hovers interactive overlay elements.

## 3. Fast In-Game Augment Card Detection (Win32 BitBlt + Native Windows OCR)

Detects offered augment round cards with low CPU overhead and no file I/O.

- **Capture Pipeline**:
  - GDI `BitBlt` captures game window client DC directly in memory (`<10ms`).
  - Crops 3 augment card title regions at fixed relative coordinates:
    - Left Card: `21.0%`
    - Center Card: `41.5%`
    - Right Card: `62.0%`
- **In-Memory OCR**:
  - Raw pixel buffers wrap into `windows::Media::Ocr::OcrEngine` using `SoftwareBitmap` (`BitmapPixelFormat::Bgra8`).
  - Runs native Windows OCR without disk writes or external Tesseract binaries.
- **Fuzzy Augment Matching**:
  - Extracted text lines match against active champion augment pool from `get_mayhem_augments`.
  - Uses token-based Levenshtein distance to resolve OCR typos, champion name variants, and line breaks.
  - Context filters rank matches by champion affinity and already selected augments.

## 4. In-Game Mayhem Overlay HUD (`LiveMayhemOverlay.svelte`)

Presentation layer rendered inside the transparent overlay window.

### Card Choice Overlays
- Aligns directly above the 3 in-game augment offer cards.
- Displays `#1 BEST PICK` recommendation badge on highest value pick.
- Shows Bayesian WPA, Win Rate (WR), Pick Rate, and sample size (`games`).
- 1-click "Select Augment" button records selection into active build state.

### Top-Right HUD Pill & Drawer
- Compact status pill in top-right screen area; expands on hover/click into full advisor drawer.
- **Live Shop & Stage Advisor**:
  - Evaluates current gold vs live game time.
  - Lists immediate affordable components/items.
  - Displays gold deficit required for next target power-spike item.
- **Core Build & Augment Slots**:
  - 6-slot Core Build path showing completed and planned items.
  - 4 active augment slots with manual augment quick-picker fallback.
- **Mayhem Combos**:
  - Surfaces top mined synergies ranked by `synergy_score` based on picked augments and items.

### Bidirectional State Synchronization
- Listens and emits Tauri event `"overlay:state-sync"`.
- Synchronizes picked champion, selected augments, active items, and game phase between `MayhemChampSelectView.svelte` and `LiveMayhemOverlay.svelte`.
- Ensures seamless state transition from Champion Select to in-game match.

## 5. Known Issues, Diagnostics & Next Steps

### Test Symptom
- Opening overlay with active League client causes empty black screen with mouse cursor, occluding game view until LCU Companion exits.

### Technical Diagnosis
- Full-screen transparent WebView2 window with `WS_EX_TOPMOST | WS_EX_LAYERED` under DirectX borderless/fullscreen causes DWM occlusion or DirectComposition black-surface rasterization when covering active DXGI swapchains.
- Child HWND enumeration toggle can conflict with Chromium GPU process compositor.

### Planned Fixes (Next Iteration)
- Transition from 1 giant full-screen transparent window to small localized widget windows (separate compact card window + HUD pill window).
- Investigate Chromium flags (`--disable-direct-composition`, `--disable-gpu-compositing`, `--enable-features=AllowUndecoratedTransparentWindows`).
- Direct2D / lightweight native window alternative if WebView2 composition remains problematic over fullscreen DirectX.

