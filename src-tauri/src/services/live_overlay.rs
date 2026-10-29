use crate::models::history::{MayhemAugmentItem, MayhemAugmentsQuery, OverlayAugmentCard};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

pub static OVERLAY_ENABLED: AtomicBool = AtomicBool::new(false);
pub static OVERLAY_HAS_CARDS: AtomicBool = AtomicBool::new(false);
static F8_WAS_DOWN: AtomicBool = AtomicBool::new(false);
static F7_WAS_DOWN: AtomicBool = AtomicBool::new(false);

pub fn poll_overlay_hotkeys() -> (bool, bool) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let f8_down = (unsafe { GetAsyncKeyState(0x77) } as u16 & 0x8000) != 0;
        let f7_down = (unsafe { GetAsyncKeyState(0x76) } as u16 & 0x8000) != 0;

        let prev_f8 = F8_WAS_DOWN.swap(f8_down, Ordering::Relaxed);
        let prev_f7 = F7_WAS_DOWN.swap(f7_down, Ordering::Relaxed);

        (f8_down && !prev_f8, f7_down && !prev_f7)
    }
    #[cfg(not(windows))]
    {
        (false, false)
    }
}

pub fn is_overlay_enabled() -> bool {
    OVERLAY_ENABLED.load(Ordering::Relaxed)
}

pub fn get_league_window_rect() -> Option<GameWindowRect> {
    let r = get_league_game_window_rect();
    if r.found {
        Some(r)
    } else {
        None
    }
}

pub fn update_overlay_ui_state(payload: &serde_json::Value) {
    if let Some(b) = payload.get("hasCards").and_then(|v| v.as_bool()) {
        OVERLAY_HAS_CARDS.store(b, Ordering::Relaxed);
    } else if let Some(arr) = payload.get("offeredAugments").and_then(|v| v.as_array()) {
        OVERLAY_HAS_CARDS.store(!arr.is_empty(), Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWindowRect {
    pub found: bool,
    pub is_foreground: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Default for GameWindowRect {
    fn default() -> Self {
        Self {
            found: false,
            is_foreground: false,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

pub fn get_league_game_window_rect() -> GameWindowRect {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::{POINT, RECT};
        use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowW, GetClientRect, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
            IsIconic, IsWindowVisible,
        };

        let title_wide: Vec<u16> = OsStr::new("League of Legends (TM) Client")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), title_wide.as_ptr());
            if hwnd.is_null() || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
                return GameWindowRect::default();
            }

            let mut rc = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetClientRect(hwnd, &mut rc) == 0 {
                return GameWindowRect::default();
            }

            let width = rc.right - rc.left;
            let height = rc.bottom - rc.top;

            if width <= 100 || height <= 100 {
                return GameWindowRect::default();
            }

            let mut pt = POINT { x: 0, y: 0 };
            ClientToScreen(hwnd, &mut pt);

            let fg = GetForegroundWindow();
            let mut is_foreground = fg == hwnd;

            if !is_foreground && !fg.is_null() {
                let len = GetWindowTextLengthW(fg);
                if len > 0 {
                    let mut buf: Vec<u16> = vec![0; (len + 1) as usize];
                    let read_len = GetWindowTextW(fg, buf.as_mut_ptr(), len + 1);
                    if read_len > 0 {
                        let text = String::from_utf16_lossy(&buf[..read_len as usize]);
                        let lower = text.to_lowercase();
                        if lower.contains("lcu companion") || lower.contains("overlay") {
                            is_foreground = true;
                        }
                    }
                }
            }

            GameWindowRect {
                found: true,
                is_foreground,
                x: pt.x,
                y: pt.y,
                width,
                height,
            }
        }
    }
    #[cfg(not(windows))]
    {
        GameWindowRect::default()
    }
}

#[inline]
fn overlay_safe_height(h: i32) -> i32 {
    (h - 1).max(1)
}

#[cfg(windows)]
pub fn apply_overlay_win32_styles(window: &tauri::WebviewWindow, _interactive: bool) {
    let _ = window.set_ignore_cursor_events(true);

    if let Ok(hwnd_val) = window.hwnd() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_APPWINDOW, WS_EX_LAYERED,
            WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
        };

        let hwnd = hwnd_val.0 as windows_sys::Win32::Foundation::HWND;
        unsafe {
            let cur = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let target = (cur
                | WS_EX_LAYERED
                | WS_EX_TRANSPARENT
                | WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE)
                & !WS_EX_APPWINDOW;

            if cur != target {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target as isize);
            }
        }
    }
}

#[cfg(not(windows))]
pub fn apply_overlay_win32_styles(window: &tauri::WebviewWindow, _interactive: bool) {
    let _ = window.set_ignore_cursor_events(true);
}

pub fn show_overlay_banner_rect(app: &tauri::AppHandle, game_rect: &GameWindowRect) {
    use tauri::Manager;
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };

    let x = game_rect.x;
    let y = game_rect.y;
    let w = game_rect.width.saturating_sub(1).max(1);
    let h = game_rect.height.saturating_sub(1).max(1);

    apply_overlay_win32_styles(&window, false);

    #[cfg(windows)]
    {
        if let Ok(hwnd_val) = window.hwnd() {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW,
                SW_SHOWNOACTIVATE,
            };
            let hwnd = hwnd_val.0 as windows_sys::Win32::Foundation::HWND;
            unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    x,
                    y,
                    w,
                    h,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            return;
        }
    }

    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
        width: w.max(1) as u32,
        height: h.max(1) as u32,
    }));
    let _ = window.show();
}

pub fn hide_overlay_window(app: &tauri::AppHandle) {
    use tauri::Manager;
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };

    #[cfg(windows)]
    {
        if let Ok(hwnd_val) = window.hwnd() {
            use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
            let hwnd = hwnd_val.0 as windows_sys::Win32::Foundation::HWND;
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
            }
            return;
        }
    }

    let _ = window.hide();
}

pub fn sync_overlay_window(
    app: &tauri::AppHandle,
    enabled: bool,
    interactive: bool,
) -> Result<GameWindowRect, String> {
    use tauri::Manager;

    OVERLAY_ENABLED.store(enabled, Ordering::Relaxed);

    let rect = get_league_game_window_rect();
    let window = match app.get_webview_window("overlay") {
        Some(w) => w,
        None => return Ok(rect),
    };

    if !enabled || !rect.found {
        hide_overlay_window(app);
        return Ok(rect);
    }

    let safe_h = overlay_safe_height(rect.height);

    #[cfg(windows)]
    {
        if let Ok(hwnd_val) = window.hwnd() {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW,
                SW_HIDE, SW_SHOWNOACTIVATE,
            };

            let hwnd = hwnd_val.0 as windows_sys::Win32::Foundation::HWND;
            unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    rect.x,
                    rect.y,
                    rect.width,
                    safe_h,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
            apply_overlay_win32_styles(&window, !interactive);
            if rect.is_foreground {
                unsafe {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
            } else {
                unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                }
            }
            return Ok(rect);
        }
    }

    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
        x: rect.x,
        y: rect.y,
    }));
    let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
        width: rect.width as u32,
        height: safe_h as u32,
    }));
    let _ = window.set_always_on_top(true);
    apply_overlay_win32_styles(&window, !interactive);
    if rect.is_foreground {
        let _ = window.show();
    } else {
        let _ = window.hide();
    }

    Ok(rect)
}

#[cfg(windows)]
fn capture_bgra_rect(
    sx: i32,
    sy: i32,
    sw: i32,
    sh: i32,
) -> Result<(Vec<u8>, u32, u32), String> {
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };

    if sw <= 0 || sh <= 0 {
        return Err("Invalid capture dimensions".to_string());
    }

    unsafe {
        let hdc_screen = GetDC(std::ptr::null_mut());
        if hdc_screen.is_null() {
            return Err("Failed to get screen DC".to_string());
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_null() {
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return Err("Failed to create compatible DC".to_string());
        }

        let hbm = CreateCompatibleBitmap(hdc_screen, sw, sh);
        if hbm.is_null() {
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return Err("Failed to create compatible bitmap".to_string());
        }

        let old_bmp = SelectObject(hdc_mem, hbm);
        let blt_res = BitBlt(hdc_mem, 0, 0, sw, sh, hdc_screen, sx, sy, SRCCOPY);
        SelectObject(hdc_mem, old_bmp);

        if blt_res == 0 {
            DeleteObject(hbm);
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return Err("BitBlt failed".to_string());
        }

        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: sw,
                biHeight: -sh, // Top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: (sw * sh * 4) as u32,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [windows_sys::Win32::Graphics::Gdi::RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };

        let mut buf = vec![0u8; (sw * sh * 4) as usize];
        let get_di = GetDIBits(
            hdc_mem,
            hbm,
            0,
            sh as u32,
            buf.as_mut_ptr() as *mut _,
            &mut bi,
            DIB_RGB_COLORS,
        );

        DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);

        if get_di == 0 {
            return Err("GetDIBits failed".to_string());
        }

        Ok((buf, sw as u32, sh as u32))
    }
}

#[cfg(windows)]
async fn recognize_text_lines(
    bgra_bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<(f32, String)>, String> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;

    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| format!("OCR engine init failed: {e}"))?;

    let writer = DataWriter::new().map_err(|e| format!("DataWriter create failed: {e}"))?;
    writer
        .WriteBytes(bgra_bytes)
        .map_err(|e| format!("DataWriter WriteBytes failed: {e}"))?;
    let buffer = writer
        .DetachBuffer()
        .map_err(|e| format!("DetachBuffer failed: {e}"))?;

    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        width as i32,
        height as i32,
    )
    .map_err(|e| format!("SoftwareBitmap creation failed: {e}"))?;

    let ocr_op = engine
        .RecognizeAsync(&bitmap)
        .map_err(|e| format!("RecognizeAsync call failed: {e}"))?;
    let ocr_result = ocr_op
        .get()
        .map_err(|e| format!("OCR recognize failed: {e}"))?;

    let lines = ocr_result
        .Lines()
        .map_err(|e| format!("Lines access failed: {e}"))?;
    let count = lines.Size().unwrap_or(0);

    let mut result = Vec::new();
    for i in 0..count {
        if let Ok(line) = lines.GetAt(i) {
            let text = line.Text().map(|h| h.to_string()).unwrap_or_default();
            let mut rel_x = 0.5f32;
            if let Ok(words) = line.Words() {
                if let Ok(w_cnt) = words.Size() {
                    if w_cnt > 0 && width > 0 {
                        let mut min_x = f32::MAX;
                        let mut max_x = f32::MIN;
                        let mut found_word = false;
                        for w_idx in 0..w_cnt {
                            if let Ok(w) = words.GetAt(w_idx) {
                                if let Ok(rect) = w.BoundingRect() {
                                    min_x = min_x.min(rect.X);
                                    max_x = max_x.max(rect.X + rect.Width);
                                    found_word = true;
                                }
                            }
                        }
                        if found_word {
                            rel_x = ((min_x + max_x) * 0.5) / (width as f32);
                        } else if let Ok(first_word) = words.GetAt(0) {
                            if let Ok(rect) = first_word.BoundingRect() {
                                rel_x = (rect.X + rect.Width * 0.5) / (width as f32);
                            }
                        }
                    }
                }
            }
            if !text.trim().is_empty() {
                result.push((rel_x, text));
            }
        }
    }

    Ok(result)
}

fn normalize_text(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn text_similarity_score(candidate: &str, target: &str) -> f64 {
    let c = normalize_text(candidate);
    let t = normalize_text(target);
    if c.is_empty() || t.is_empty() {
        return 0.0;
    }
    if c.contains(&t) || t.contains(&c) {
        return 1.0;
    }
    let c_words: Vec<&str> = c.split_whitespace().collect();
    let t_words: Vec<&str> = t.split_whitespace().collect();
    if c_words.is_empty() || t_words.is_empty() {
        return 0.0;
    }
    let mut matches = 0;
    for cw in &c_words {
        if t_words.iter().any(|tw| tw == cw || tw.contains(cw) || cw.contains(tw)) {
            matches += 1;
        }
    }
    (matches as f64) / (t_words.len().max(c_words.len()) as f64)
}

pub fn is_category_badge_noise(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    matches!(
        lower.as_str(),
        "damage" | "utility" | "resilience" | "speed" | "quest"
    )
}

pub fn match_ocr_lines_to_augments(
    lines: &[(f32, String)],
    all_augments: &[MayhemAugmentItem],
) -> Vec<(usize, MayhemAugmentItem)> {
    let mut columns: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];

    for (rel_x, text) in lines {
        let trimmed = text.trim();
        if trimmed.is_empty() || is_category_badge_noise(trimmed) {
            continue;
        }

        let col = if *rel_x < 0.333 {
            0
        } else if *rel_x < 0.666 {
            1
        } else {
            2
        };
        columns[col].push(trimmed.to_string());
    }

    let mut matches = Vec::new();

    for (col_idx, col_lines) in columns.iter().enumerate() {
        if col_lines.is_empty() {
            continue;
        }

        let mut best_score = 0.0;
        let mut best_match: Option<MayhemAugmentItem> = None;

        for line in col_lines {
            let line_norm = normalize_text(line);
            if line_norm.is_empty() {
                continue;
            }

            for aug in all_augments {
                let aug_norm = normalize_text(&aug.name);
                if aug_norm.is_empty() {
                    continue;
                }

                let score = if line_norm == aug_norm {
                    1.0
                } else if line_norm.contains(&aug_norm) || aug_norm.contains(&line_norm) {
                    0.95
                } else {
                    text_similarity_score(&line_norm, &aug_norm)
                };

                if score >= 0.60 && score > best_score {
                    best_score = score;
                    best_match = Some(aug.clone());
                }
            }
        }

        if let Some(matched) = best_match {
            matches.push((col_idx, matched));
        }
    }

    matches
}

pub async fn scan_title_band_from_image(
    img: &image::RgbaImage,
    champion_id: Option<i64>,
) -> Result<Vec<OverlayAugmentCard>, String> {
    #[cfg(not(windows))]
    {
        let _ = (img, champion_id);
        return Ok(Vec::new());
    }

    #[cfg(windows)]
    {
        let width = img.width();
        let height = img.height();
        if width == 0 || height == 0 {
            return Ok(Vec::new());
        }

        // Convert RGBA to BGRA
        let mut bgra = Vec::with_capacity((width * height * 4) as usize);
        for pixel in img.pixels() {
            bgra.push(pixel[2]); // B
            bgra.push(pixel[1]); // G
            bgra.push(pixel[0]); // R
            bgra.push(pixel[3]); // A
        }

        let lines = recognize_text_lines(&bgra, width, height).await?;
        let resp = crate::services::history::get_mayhem_augments(MayhemAugmentsQuery {
            champion_id,
            page_size: Some(300),
            ..Default::default()
        })
        .await?;

        let matches = match_ocr_lines_to_augments(&lines, &resp.items);
        if matches.len() < 2 {
            return Ok(Vec::new());
        }

        Ok(crate::services::history::enrich_overlay_augment_cards(champion_id, matches).await)
    }
}

pub async fn scan_in_game_augment_cards(
    champion_id: Option<i64>,
    with_augment_ids: Vec<String>,
) -> Result<Vec<OverlayAugmentCard>, String> {
    #[cfg(not(windows))]
    {
        let _ = (champion_id, with_augment_ids);
        return Ok(Vec::new());
    }

    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::{POINT, RECT};
        use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowW, GetClientRect, GetForegroundWindow, IsIconic, IsWindowVisible,
        };

        let title_wide: Vec<u16> = OsStr::new("League of Legends (TM) Client")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let (hwnd, rect) = unsafe {
            let hwnd = FindWindowW(std::ptr::null(), title_wide.as_ptr());
            if hwnd.is_null() || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
                return Ok(Vec::new());
            }

            let fg = GetForegroundWindow();
            if fg != hwnd {
                return Ok(Vec::new());
            }

            let mut rc = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetClientRect(hwnd, &mut rc) == 0 {
                return Ok(Vec::new());
            }

            let width = rc.right - rc.left;
            let height = rc.bottom - rc.top;
            if width <= 100 || height <= 100 {
                return Ok(Vec::new());
            }

            let mut pt = POINT { x: 0, y: 0 };
            ClientToScreen(hwnd, &mut pt);

            (
                hwnd,
                GameWindowRect {
                    found: true,
                    is_foreground: true,
                    x: pt.x,
                    y: pt.y,
                    width,
                    height,
                },
            )
        };

        let _ = hwnd;

        let strip_x = rect.x + (rect.width as f64 * 0.21) as i32;
        let strip_y = rect.y + (rect.height as f64 * 0.385) as i32;
        let strip_w = (rect.width as f64 * 0.58) as i32;
        let strip_h = (rect.height as f64 * 0.065) as i32;

        let (bgra, bw, bh) = capture_bgra_rect(strip_x, strip_y, strip_w, strip_h)?;
        let lines = recognize_text_lines(&bgra, bw, bh).await?;

        let resp = crate::services::history::get_mayhem_augments(MayhemAugmentsQuery {
            champion_id,
            page_size: Some(300),
            with_augment_ids: Some(with_augment_ids),
            ..Default::default()
        })
        .await?;

        let matches = match_ocr_lines_to_augments(&lines, &resp.items);
        if matches.len() < 2 {
            return Ok(Vec::new());
        }

        Ok(crate::services::history::enrich_overlay_augment_cards(champion_id, matches).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_category_badge_noise() {
        assert!(is_category_badge_noise("Damage"));
        assert!(is_category_badge_noise("UTILITY"));
        assert!(is_category_badge_noise(" Resilience "));
        assert!(is_category_badge_noise("speed"));
        assert!(is_category_badge_noise("quest"));
        assert!(!is_category_badge_noise("Giant Slayer"));
    }

    #[test]
    fn test_match_ocr_lines_to_augments() {
        let fake_augments = vec![
            MayhemAugmentItem {
                id: "1".into(),
                name: "Phenomenal Evil".into(),
                icon_url: "".into(),
                rarity: Some("Gold".into()),
                tooltip: None,
                categories: vec![],
                games: 10,
                wins: 5,
                win_rate: 0.5,
                wpa: 0.02,
                pick_rate: None,
                global_games: 100,
                global_win_rate: 0.5,
                global_wpa: 0.02,
                global_pick_rate: None,
            },
            MayhemAugmentItem {
                id: "2".into(),
                name: "Eureka".into(),
                icon_url: "".into(),
                rarity: Some("Prismatic".into()),
                tooltip: None,
                categories: vec![],
                games: 10,
                wins: 5,
                win_rate: 0.5,
                wpa: 0.05,
                pick_rate: None,
                global_games: 100,
                global_win_rate: 0.5,
                global_wpa: 0.05,
                global_pick_rate: None,
            },
        ];

        let lines = vec![
            (0.1f32, "Damage".to_string()),
            (0.2f32, "Phenomenal Evil".to_string()),
            (0.5f32, "Utility".to_string()),
            (0.55f32, "Eureka".to_string()),
        ];

        let matched = match_ocr_lines_to_augments(&lines, &fake_augments);
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].0, 0);
        assert_eq!(matched[0].1.name, "Phenomenal Evil");
        assert_eq!(matched[1].0, 1);
        assert_eq!(matched[1].1.name, "Eureka");
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn test_scan_title_band_from_example_image() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(manifest_dir)
            .parent()
            .unwrap()
            .join("@llm")
            .join("cache")
            .join("augment-selection-examples")
            .join("Screen15.png");

        if !path.exists() {
            eprintln!("Skipping test, image file not found: {:?}", path);
            return;
        }

        let dynamic_img = match image::open(&path) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("Failed to open test image: {e}");
                return;
            }
        };

        let rgba = dynamic_img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();

        // Crop title strip: rx = 0.21, ry = 0.385, rw = 0.58, rh = 0.065
        let strip_x = (width as f64 * 0.21) as u32;
        let strip_y = (height as f64 * 0.385) as u32;
        let strip_w = (width as f64 * 0.58) as u32;
        let strip_h = (height as f64 * 0.065) as u32;

        let cropped = image::imageops::crop_imm(&rgba, strip_x, strip_y, strip_w, strip_h).to_image();
        let (cw, ch) = cropped.dimensions();
        assert!(cw > 0 && ch > 0);

        let mut bgra = Vec::with_capacity((cw * ch * 4) as usize);
        for pixel in cropped.pixels() {
            bgra.push(pixel[2]);
            bgra.push(pixel[1]);
            bgra.push(pixel[0]);
            bgra.push(pixel[3]);
        }

        let lines = recognize_text_lines(&bgra, cw, ch).await;
        match lines {
            Ok(lines) => {
                println!("OCR detected {} lines from Screen15.png title band:", lines.len());
                for (rel_x, text) in &lines {
                    println!("  [{:.2}] {}", rel_x, text);
                }
                assert!(!lines.is_empty(), "Expected OCR to find title band lines");
            }
            Err(e) => {
                eprintln!("OCR recognition failed in test environment: {e}");
            }
        }
    }
}
