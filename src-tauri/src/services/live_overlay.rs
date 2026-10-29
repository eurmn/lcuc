use crate::models::history::{MayhemAugmentItem, MayhemAugmentsQuery};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub static OVERLAY_ENABLED: AtomicBool = AtomicBool::new(false);
pub static OVERLAY_HAS_CARDS: AtomicBool = AtomicBool::new(false);
pub static OVERLAY_HUD_EXPANDED: AtomicBool = AtomicBool::new(true);
pub static OVERLAY_CURSOR_THREAD_STARTED: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    pub static ref OVERLAY_INTERACTIVE_RECTS: Mutex<Vec<(i32, i32, i32, i32)>> = Mutex::new(Vec::new());
}

pub fn update_overlay_ui_state(payload: &serde_json::Value) {
    if let Some(b) = payload.get("hasCards").and_then(|v| v.as_bool()) {
        OVERLAY_HAS_CARDS.store(b, Ordering::Relaxed);
    } else if let Some(arr) = payload.get("offeredAugments").and_then(|v| v.as_array()) {
        OVERLAY_HAS_CARDS.store(!arr.is_empty(), Ordering::Relaxed);
    }

    if let Some(b) = payload.get("hudExpanded").and_then(|v| v.as_bool()) {
        OVERLAY_HUD_EXPANDED.store(b, Ordering::Relaxed);
    }

    if let Some(arr) = payload.get("interactiveRects").and_then(|v| v.as_array()) {
        let mut rects = Vec::new();
        for item in arr {
            let x = item.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = item.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let width = item.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let height = item.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            rects.push((x, y, width, height));
        }
        if let Ok(mut lock) = OVERLAY_INTERACTIVE_RECTS.lock() {
            *lock = rects;
        }
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

#[cfg(windows)]
unsafe extern "system" fn enum_child_proc(
    child: windows_sys::Win32::Foundation::HWND,
    lparam: isize,
) -> i32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TRANSPARENT,
    };

    let cur = GetWindowLongPtrW(child, GWL_EXSTYLE) as u32;
    let mut target = cur | WS_EX_LAYERED | WS_EX_NOACTIVATE;
    if lparam != 0 {
        target |= WS_EX_TRANSPARENT;
    } else {
        target &= !WS_EX_TRANSPARENT;
    }
    if cur != target {
        SetWindowLongPtrW(child, GWL_EXSTYLE, target as isize);
    }
    1
}

#[cfg(windows)]
pub fn apply_overlay_win32_styles(window: &tauri::WebviewWindow, ignore_cursor: bool) {
    let _ = window.set_ignore_cursor_events(ignore_cursor);

    if let Ok(hwnd_val) = window.hwnd() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            EnumChildWindows, GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_LAYERED,
            WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
        };

        let hwnd = hwnd_val.0 as windows_sys::Win32::Foundation::HWND;
        unsafe {
            let cur = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let mut target =
                cur | WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
            if ignore_cursor {
                target |= WS_EX_TRANSPARENT;
            } else {
                target &= !WS_EX_TRANSPARENT;
            }

            if cur != target {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target as isize);
            }

            EnumChildWindows(
                hwnd,
                Some(enum_child_proc),
                if ignore_cursor { 1 } else { 0 },
            );
        }
    }
}

#[cfg(not(windows))]
pub fn apply_overlay_win32_styles(window: &tauri::WebviewWindow, ignore_cursor: bool) {
    let _ = window.set_ignore_cursor_events(ignore_cursor);
}

pub fn ensure_overlay_cursor_thread(app: tauri::AppHandle) {
    if OVERLAY_CURSOR_THREAD_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    std::thread::spawn(move || {
        use tauri::Manager;
        let mut last_ignore: Option<bool> = None;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(40));

            if !OVERLAY_ENABLED.load(Ordering::Relaxed) {
                last_ignore = None;
                continue;
            }

            let rect = get_league_game_window_rect();
            let window = match app.get_webview_window("overlay") {
                Some(w) => w,
                None => continue,
            };

            if !rect.found || !rect.is_foreground {
                #[cfg(windows)]
                {
                    if let Ok(hwnd_val) = window.hwnd() {
                        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
                        unsafe {
                            ShowWindow(hwnd_val.0 as windows_sys::Win32::Foundation::HWND, SW_HIDE);
                        }
                    } else {
                        let _ = window.hide();
                    }
                }
                #[cfg(not(windows))]
                {
                    let _ = window.hide();
                }
                last_ignore = None;
                continue;
            }

            #[cfg(windows)]
            {
                if let Ok(hwnd_val) = window.hwnd() {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        GetCursorPos, SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE,
                        SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
                    };

                    let hwnd = hwnd_val.0 as windows_sys::Win32::Foundation::HWND;
                    unsafe {
                        SetWindowPos(
                            hwnd,
                            HWND_TOPMOST,
                            rect.x,
                            rect.y,
                            rect.width,
                            rect.height,
                            SWP_NOACTIVATE | SWP_SHOWWINDOW,
                        );
                        ShowWindow(hwnd, SW_SHOWNOACTIVATE);

                        let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
                        let got_cursor = GetCursorPos(&mut pt) != 0;

                        let mut over_interactive = false;
                        if got_cursor {
                            let rx = pt.x - rect.x;
                            let ry = pt.y - rect.y;

                            let rects = OVERLAY_INTERACTIVE_RECTS
                                .lock()
                                .map(|l| l.clone())
                                .unwrap_or_default();

                            if !rects.is_empty() {
                                for (x, y, w, h) in rects {
                                    if rx >= x && rx < x + w && ry >= y && ry < y + h {
                                        over_interactive = true;
                                        break;
                                    }
                                }
                            } else {
                                let hud_height = if OVERLAY_HUD_EXPANDED.load(Ordering::Relaxed) {
                                    520
                                } else {
                                    64
                                };
                                if rx >= (rect.width - 360) && ry <= hud_height {
                                    over_interactive = true;
                                }
                            }
                        }

                        let target_ignore = !over_interactive;
                        if last_ignore != Some(target_ignore) {
                            apply_overlay_win32_styles(&window, target_ignore);
                            last_ignore = Some(target_ignore);
                        }
                    }
                }
            }

            #[cfg(not(windows))]
            {
                let _ = window.show();
            }
        }
    });
}

pub fn sync_overlay_window(
    app: &tauri::AppHandle,
    enabled: bool,
    interactive: bool,
) -> Result<GameWindowRect, String> {
    use tauri::Manager;

    OVERLAY_ENABLED.store(enabled, Ordering::Relaxed);
    ensure_overlay_cursor_thread(app.clone());

    let rect = get_league_game_window_rect();
    let window = match app.get_webview_window("overlay") {
        Some(w) => w,
        None => return Ok(rect),
    };

    if !enabled || !rect.found {
        #[cfg(windows)]
        {
            if let Ok(hwnd_val) = window.hwnd() {
                use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
                unsafe {
                    ShowWindow(hwnd_val.0 as windows_sys::Win32::Foundation::HWND, SW_HIDE);
                }
            } else {
                let _ = window.hide();
            }
        }
        #[cfg(not(windows))]
        {
            let _ = window.hide();
        }
        return Ok(rect);
    }

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
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
            apply_overlay_win32_styles(&window, !interactive);

            if rect.is_foreground {
                unsafe {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
            } else {
                use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;
                unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                }
            }
        } else {
            let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: rect.x,
                y: rect.y,
            }));
            let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: rect.width as u32,
                height: rect.height as u32,
            }));
            let _ = window.set_always_on_top(true);
            apply_overlay_win32_styles(&window, !interactive);
            if rect.is_foreground {
                let _ = window.show();
            } else {
                let _ = window.hide();
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: rect.x,
            y: rect.y,
        }));
        let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: rect.width as u32,
            height: rect.height as u32,
        }));
        let _ = window.set_always_on_top(true);
        apply_overlay_win32_styles(&window, !interactive);
        if rect.is_foreground {
            let _ = window.show();
        } else {
            let _ = window.hide();
        }
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
) -> Result<Vec<(f64, String)>, String> {
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
            let mut rel_x = 0.5;
            if let Ok(words) = line.Words() {
                if let Ok(w_cnt) = words.Size() {
                    if w_cnt > 0 {
                        if let Ok(first_word) = words.GetAt(0) {
                            if let Ok(rect) = first_word.BoundingRect() {
                                if width > 0 {
                                    rel_x = rect.X as f64 / width as f64;
                                }
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

pub async fn scan_in_game_augment_cards(
    champion_id: Option<i64>,
    with_augment_ids: Vec<String>,
) -> Result<Vec<MayhemAugmentItem>, String> {
    #[cfg(not(windows))]
    {
        let _ = (champion_id, with_augment_ids);
        return Ok(Vec::new());
    }

    #[cfg(windows)]
    {
        let rect = get_league_game_window_rect();
        if !rect.found || rect.width <= 100 || rect.height <= 100 {
            return Err("League game window not found or minimized".to_string());
        }

        // Augment selection popup strip: rx = 0.18, ry = 0.20, rw = 0.64, rh = 0.42
        let strip_x = rect.x + (rect.width as f64 * 0.18) as i32;
        let strip_y = rect.y + (rect.height as f64 * 0.20) as i32;
        let strip_w = (rect.width as f64 * 0.64) as i32;
        let strip_h = (rect.height as f64 * 0.42) as i32;

        let (bgra, bw, bh) = capture_bgra_rect(strip_x, strip_y, strip_w, strip_h)?;
        let lines = recognize_text_lines(&bgra, bw, bh).await?;

        let mut col_texts = [String::new(), String::new(), String::new()];
        for (rel_x, text) in lines {
            let col = if rel_x < 0.33 {
                0
            } else if rel_x < 0.66 {
                1
            } else {
                2
            };
            col_texts[col].push(' ');
            col_texts[col].push_str(&text);
        }

        let resp = crate::services::history::get_mayhem_augments(MayhemAugmentsQuery {
            champion_id,
            page_size: Some(300),
            with_augment_ids: Some(with_augment_ids),
            ..Default::default()
        })
        .await?;

        let all_augments = resp.items;
        let mut results = Vec::new();

        for text in &col_texts {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }

            let mut best_score = 0.0;
            let mut best_match: Option<MayhemAugmentItem> = None;

            for aug in &all_augments {
                let score = text_similarity_score(trimmed, &aug.name);
                if score > best_score && score >= 0.35 {
                    best_score = score;
                    best_match = Some(aug.clone());
                }
            }

            if let Some(matched) = best_match {
                if !results.iter().any(|r: &MayhemAugmentItem| r.id == matched.id) {
                    results.push(matched);
                }
            }
        }

        Ok(results)
    }
}
