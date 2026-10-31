use crate::models::history::{MayhemAugmentItem, MayhemAugmentsQuery, OverlayAugmentCard};
use crate::services::native_overlay::OverlayPngSprite;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub static OVERLAY_ENABLED: AtomicBool = AtomicBool::new(true);
pub static OVERLAY_HAS_CARDS: AtomicBool = AtomicBool::new(false);
pub static OVERLAY_SCANNING: AtomicBool = AtomicBool::new(false);
static F8_WAS_DOWN: AtomicBool = AtomicBool::new(false);
static F7_WAS_DOWN: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    static ref OVERLAY_CHAMPION_NAME: Mutex<String> = Mutex::new(String::new());
    static ref OVERLAY_STRATEGY_LABEL: Mutex<String> = Mutex::new(String::new());
    static ref OVERLAY_OFFERED_CARDS: Mutex<Vec<OverlayAugmentCard>> = Mutex::new(Vec::new());
    pub static ref OVERLAY_RECOMMENDED_SPRITES: Mutex<Vec<OverlayPngSprite>> = Mutex::new(Vec::new());
    static ref OVERLAY_STATUS_TOAST: Mutex<Option<(String, bool, std::time::Instant)>> = Mutex::new(None);
}

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

pub fn set_overlay_scanning(scanning: bool) {
    OVERLAY_SCANNING.store(scanning, Ordering::Relaxed);
    refresh_native_overlay();
}

pub fn update_overlay_cards(cards: &[OverlayAugmentCard]) {
    if let Ok(mut lock) = OVERLAY_OFFERED_CARDS.lock() {
        *lock = cards.to_vec();
    }
    OVERLAY_HAS_CARDS.store(!cards.is_empty(), Ordering::Relaxed);
    refresh_native_overlay();
}

pub fn clear_overlay_cards() {
    update_overlay_cards(&[]);
}

pub fn clear_overlay_recommended_sprites() {
    if let Ok(mut lock) = OVERLAY_RECOMMENDED_SPRITES.lock() {
        lock.clear();
    }
    refresh_native_overlay();
}

pub fn set_overlay_champion_and_strategy(champ: &str, strat: &str) {
    let mut changed = false;
    if let Ok(mut c) = OVERLAY_CHAMPION_NAME.lock() {
        if *c != champ {
            *c = champ.to_string();
            changed = true;
        }
    }
    if let Ok(mut s) = OVERLAY_STRATEGY_LABEL.lock() {
        if *s != strat {
            *s = strat.to_string();
            changed = true;
        }
    }
    if changed {
        refresh_native_overlay();
    }
}

pub fn show_overlay_status_toast(message: &str, is_error: bool) {
    if let Ok(mut lock) = OVERLAY_STATUS_TOAST.lock() {
        *lock = Some((message.to_string(), is_error, std::time::Instant::now()));
    }
    refresh_native_overlay();
}

pub fn refresh_native_overlay() {
    if !is_overlay_enabled() {
        return;
    }
    let champ = OVERLAY_CHAMPION_NAME.lock().map(|g| g.clone()).unwrap_or_default();
    let strat = OVERLAY_STRATEGY_LABEL.lock().map(|g| g.clone()).unwrap_or_default();
    let cards = OVERLAY_OFFERED_CARDS.lock().map(|g| g.clone()).unwrap_or_default();
    let sprites = OVERLAY_RECOMMENDED_SPRITES.lock().map(|g| g.clone()).unwrap_or_default();
    let scanning = OVERLAY_SCANNING.load(Ordering::Relaxed);

    let (status_message, is_error) = if let Ok(mut lock) = OVERLAY_STATUS_TOAST.lock() {
        if let Some((msg, err, instant)) = lock.as_ref() {
            if instant.elapsed() < std::time::Duration::from_secs(3) {
                (Some(msg.clone()), *err)
            } else {
                *lock = None;
                (None, false)
            }
        } else {
            (None, false)
        }
    } else {
        (None, false)
    };

    let payload = crate::services::native_overlay::OverlayRenderPayload {
        visible: true,
        scanning,
        champion_name: champ,
        strategy_label: strat,
        cards,
        sprites,
        status_message,
        is_error,
    };
    let _ = crate::services::native_overlay::update_native_overlay_state(payload);
}

fn find_field<'a>(payload: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    if let Some(val) = payload.get(key) {
        if !val.is_null() {
            return Some(val);
        }
    }
    for sub in &["liveAdvice", "buildAdvice", "advice", "build", "recommendation", "stats"] {
        if let Some(sub_obj) = payload.get(sub) {
            if let Some(val) = sub_obj.get(key) {
                if !val.is_null() {
                    return Some(val);
                }
            }
        }
    }
    None
}

pub async fn build_recommended_item_sprites(
    boots: Option<u32>,
    next_items: Vec<u32>,
    completed_build: Vec<u32>,
) -> Vec<OverlayPngSprite> {
    let mut items_to_render: Vec<(String, u32)> = Vec::new();
    if let Some(boot_id) = boots {
        if boot_id > 0 {
            items_to_render.push(("boot".to_string(), boot_id));
        }
    }
    for id in next_items {
        if id > 0 && !items_to_render.iter().any(|(_, x)| *x == id) {
            items_to_render.push(("next".to_string(), id));
        }
    }
    for id in completed_build {
        if id > 0 && !items_to_render.iter().any(|(_, x)| *x == id) {
            items_to_render.push(("build".to_string(), id));
        }
    }

    if items_to_render.is_empty() {
        return Vec::new();
    }

    items_to_render.truncate(7);

    let rect = get_league_game_window_rect();
    let gw = if rect.found && rect.width > 100 { rect.width } else { 1920 };
    let banner_w = 340;
    let banner_h = 42;
    let banner_x = (gw - banner_w - 24).max(0);
    let banner_y = 18;

    let slot_size: u32 = 36;
    let gap = 8;
    let padding_x = 14;
    let base_y = banner_y + banner_h + 28;

    let cache_dir = crate::services::asset_cache::get_asset_cache_dir().join("items");
    let _ = tokio::fs::create_dir_all(&cache_dir).await;

    let items_catalog = crate::services::history::get_items_by_id().await.ok();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let mut sprites = Vec::with_capacity(items_to_render.len());

    for (idx, (role, item_id)) in items_to_render.into_iter().enumerate() {
        let item_path = cache_dir.join(format!("{item_id}.png"));
        let exists = match tokio::fs::metadata(&item_path).await {
            Ok(m) => m.len() > 0,
            Err(_) => false,
        };

        if !exists {
            // First try resolving the official icon_url from the items catalog
            let resolved_url = items_catalog
                .as_ref()
                .and_then(|cat| cat.get(&(item_id as i32)))
                .map(|info| info.icon_url.clone())
                .unwrap_or_else(|| {
                    format!(
                        "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/{item_id}.png"
                    )
                });

            let mut fetched = false;
            if let Ok(resp) = client.get(&resolved_url).send().await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        if !bytes.is_empty() {
                            let _ = tokio::fs::write(&item_path, &bytes).await;
                            fetched = true;
                        }
                    }
                }
            }

            // Fallback: try lowercase direct asset path if catalog url failed
            if !fetched {
                let fallback_url = format!(
                    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/{item_id}.png"
                );
                if let Ok(resp) = client.get(&fallback_url).send().await {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            if !bytes.is_empty() {
                                let _ = tokio::fs::write(&item_path, &bytes).await;
                            }
                        }
                    }
                }
            }
        }

        let png_path = if item_path.exists()
            && std::fs::metadata(&item_path).map(|m| m.len() > 0).unwrap_or(false)
        {
            Some(item_path.to_string_lossy().to_string())
        } else {
            None
        };

        let png_base64_or_bytes = if png_path.is_some() {
            tokio::fs::read(&item_path).await.ok()
        } else {
            None
        };

        let x = banner_x + padding_x + (idx as i32) * (slot_size as i32 + gap);
        let y = base_y;

        sprites.push(OverlayPngSprite {
            id: format!("item_{role}_{item_id}"),
            png_path,
            png_base64_or_bytes,
            x,
            y,
            width: slot_size,
            height: slot_size,
        });
    }

    sprites
}

pub fn notify_overlay_state_from_frontend(payload: &serde_json::Value) {
    if let Some(champ) = payload.get("championName").and_then(|v| v.as_str()) {
        if let Ok(mut c) = OVERLAY_CHAMPION_NAME.lock() {
            *c = champ.to_string();
        }
    }
    if let Some(strat) = payload
        .get("strategyName")
        .or_else(|| payload.get("strategyId"))
        .and_then(|v| v.as_str())
    {
        if let Ok(mut s) = OVERLAY_STRATEGY_LABEL.lock() {
            *s = strat.to_string();
        }
    }
    if let Some(cards_val) = payload.get("offeredAugments") {
        if let Ok(cards) = serde_json::from_value::<Vec<OverlayAugmentCard>>(cards_val.clone()) {
            if let Ok(mut lock) = OVERLAY_OFFERED_CARDS.lock() {
                *lock = cards.clone();
            }
            OVERLAY_HAS_CARDS.store(!cards.is_empty(), Ordering::Relaxed);
        }
    }
    if let Some(b) = payload.get("hasCards").and_then(|v| v.as_bool()) {
        OVERLAY_HAS_CARDS.store(b, Ordering::Relaxed);
    }

    let parse_item_id = |v: &serde_json::Value| -> Option<u32> {
        v.as_u64()
            .map(|n| n as u32)
            .or_else(|| v.as_str().and_then(|s| s.parse::<u32>().ok()))
            .or_else(|| v.get("item_id").and_then(|x| x.as_u64().map(|n| n as u32)))
            .or_else(|| v.get("itemId").and_then(|x| x.as_u64().map(|n| n as u32)))
    };

    let boots = find_field(payload, "recommendedBoots")
        .or_else(|| find_field(payload, "recommended_boots"))
        .and_then(parse_item_id);

    let extract_list = |k1: &str, k2: &str| -> Vec<u32> {
        find_field(payload, k1)
            .or_else(|| find_field(payload, k2))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(parse_item_id).collect())
            .unwrap_or_default()
    };

    let mut next_items = extract_list("nextItemIds", "next_item_ids");
    if next_items.is_empty() {
        next_items = extract_list("nextItems", "next_items");
    }
    let completed_build = extract_list("completedBuild", "completed_build");

    let has_item_keys = find_field(payload, "recommendedBoots").is_some()
        || find_field(payload, "recommended_boots").is_some()
        || find_field(payload, "nextItemIds").is_some()
        || find_field(payload, "next_item_ids").is_some()
        || find_field(payload, "nextItems").is_some()
        || find_field(payload, "next_items").is_some()
        || find_field(payload, "completedBuild").is_some()
        || find_field(payload, "completed_build").is_some();

    if has_item_keys {
        tauri::async_runtime::spawn(async move {
            let sprites = build_recommended_item_sprites(boots, next_items, completed_build).await;
            if let Ok(mut lock) = OVERLAY_RECOMMENDED_SPRITES.lock() {
                *lock = sprites;
            }
            refresh_native_overlay();
        });
    } else {
        refresh_native_overlay();
    }
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

pub fn apply_overlay_win32_styles(_window: &tauri::WebviewWindow, _interactive: bool) {}

pub fn show_overlay_banner_rect(_app: &tauri::AppHandle, game_rect: &GameWindowRect) {
    if !game_rect.found || !game_rect.is_foreground {
        crate::services::native_overlay::hide_native_overlay();
        return;
    }
    if let Err(e) = crate::services::native_overlay::ensure_overlay_process_started() {
        crate::logger::write(
            crate::logger::Level::Error,
            "live_overlay",
            &format!("Failed to start native overlay process: {e}"),
            None,
        );
        return;
    }
    crate::services::native_overlay::calibrate_if_changed(
        game_rect.x,
        game_rect.y,
        game_rect.width,
        game_rect.height,
    );
    refresh_native_overlay();
}

pub fn hide_overlay_window(_app: &tauri::AppHandle) {
    crate::services::native_overlay::hide_native_overlay();
}

pub fn sync_overlay_window(
    app: &tauri::AppHandle,
    enabled: bool,
    _interactive: bool,
) -> Result<GameWindowRect, String> {
    OVERLAY_ENABLED.store(enabled, Ordering::Relaxed);
    crate::logger::write(
        crate::logger::Level::Info,
        "live_overlay",
        &format!("sync_overlay_window: enabled={enabled}"),
        None,
    );
    let rect = get_league_game_window_rect();
    if !enabled || !rect.found {
        hide_overlay_window(app);
        return Ok(rect);
    }

    if rect.is_foreground {
        show_overlay_banner_rect(app, &rect);
    } else {
        hide_overlay_window(app);
    }

    Ok(rect)
}

#[cfg(not(windows))]
pub fn capture_bgra_rect(
    _sx: i32,
    _sy: i32,
    _sw: i32,
    _sh: i32,
) -> Result<(Vec<u8>, u32, u32), String> {
    Err("Not supported on non-windows".to_string())
}

#[cfg(windows)]
pub fn capture_bgra_rect(
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
pub async fn recognize_text_lines_public_for_test(
    bgra_bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<(f32, String)>, String> {
    recognize_text_lines(bgra_bytes, width, height).await
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

    let (scaled_bgra, final_w, final_h) = (bgra_bytes.to_vec(), width, height);

    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| format!("OCR engine init failed: {e}"))?;

    let writer = DataWriter::new().map_err(|e| format!("DataWriter create failed: {e}"))?;
    writer
        .WriteBytes(&scaled_bgra)
        .map_err(|e| format!("DataWriter WriteBytes failed: {e}"))?;
    let buffer = writer
        .DetachBuffer()
        .map_err(|e| format!("DetachBuffer failed: {e}"))?;

    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        final_w as i32,
        final_h as i32,
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
                    if w_cnt > 0 && final_w > 0 {
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
                            rel_x = ((min_x + max_x) * 0.5) / (final_w as f32);
                        } else if let Ok(first_word) = words.GetAt(0) {
                            if let Ok(rect) = first_word.BoundingRect() {
                                rel_x = (rect.X + rect.Width * 0.5) / (final_w as f32);
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

// Levenshtein edit distance for OCR misread tolerance (e.g. "Symphony of War" vs "Symphony of Wor")
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let v1: Vec<char> = s1.chars().collect();
    let v2: Vec<char> = s2.chars().collect();
    let l1 = v1.len();
    let l2 = v2.len();
    if l1 == 0 {
        return l2;
    }
    if l2 == 0 {
        return l1;
    }
    let mut prev_row: Vec<usize> = (0..=l2).collect();
    let mut curr_row: Vec<usize> = vec![0; l2 + 1];

    for i in 0..l1 {
        curr_row[0] = i + 1;
        for j in 0..l2 {
            let cost = if v1[i] == v2[j] { 0 } else { 1 };
            curr_row[j + 1] = (curr_row[j] + 1)
                .min(prev_row[j + 1] + 1)
                .min(prev_row[j] + cost);
        }
        prev_row.copy_from_slice(&curr_row);
    }
    prev_row[l2]
}

fn fuzzy_string_similarity(candidate: &str, target: &str) -> f64 {
    let c = normalize_text(candidate);
    let t = normalize_text(target);
    if c.is_empty() || t.is_empty() {
        return 0.0;
    }
    if c == t {
        return 1.0;
    }
    let max_len = c.chars().count().max(t.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    let dist = levenshtein_distance(&c, &t);
    1.0 - (dist as f64 / max_len as f64)
}

fn text_similarity_score(candidate: &str, target: &str) -> f64 {
    let c = normalize_text(candidate);
    let t = normalize_text(target);
    if c.is_empty() || t.is_empty() {
        return 0.0;
    }
    if c == t {
        return 1.0;
    }
    // High fuzzy match
    let fuzzy = fuzzy_string_similarity(&c, &t);
    if fuzzy >= 0.80 {
        return fuzzy;
    }
    if c.contains(&t) || t.contains(&c) {
        let ratio = c.len().min(t.len()) as f64 / c.len().max(t.len()) as f64;
        return (0.80 + (0.20 * ratio)).max(fuzzy);
    }
    let c_words: Vec<&str> = c.split_whitespace().collect();
    let t_words: Vec<&str> = t.split_whitespace().collect();
    if c_words.is_empty() || t_words.is_empty() {
        return 0.0;
    }
    let mut matches = 0;
    for cw in &c_words {
        if t_words.iter().any(|tw| tw == cw || fuzzy_string_similarity(tw, cw) >= 0.85) {
            matches += 1;
        }
    }
    ((matches as f64) / (t_words.len().max(c_words.len()) as f64)).max(fuzzy)
}

pub fn is_category_badge_noise(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    matches!(
        lower.as_str(),
        "damage" | "utility" | "resilience" | "speed" | "quest"
    )
}

#[cfg(windows)]
pub async fn recognize_text_lines_multi_pass(
    bgra_bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<(f32, String)>, String> {
    if width == 0 || height == 0 || bgra_bytes.len() < (width * height * 4) as usize {
        return Ok(Vec::new());
    }

    let mut combined_lines: Vec<(f32, String)> = Vec::new();
    let mut seen_lines = std::collections::HashSet::new();

    // Pass 1 & 2: Luminance binarization thresholds (130.0 and 150.0)
    // Isolates bright card titles and body text from animated card artwork backgrounds
    for threshold in [130.0f32, 150.0f32] {
        let mut bin_bgra = Vec::with_capacity((width * height * 4) as usize);
        for chunk in bgra_bytes.chunks_exact(4) {
            let b = chunk[0] as f32;
            let g = chunk[1] as f32;
            let r = chunk[2] as f32;
            let lum = 0.299 * r + 0.587 * g + 0.114 * b;
            let v = if lum > threshold { 255u8 } else { 0u8 };
            bin_bgra.push(v);
            bin_bgra.push(v);
            bin_bgra.push(v);
            bin_bgra.push(255);
        }

        if let Ok(lines) = recognize_text_lines(&bin_bgra, width, height).await {
            for (rx, text) in lines {
                let col_bucket = if rx < 0.36 { 0 } else if rx < 0.65 { 1 } else { 2 };
                let key = (col_bucket, text.trim().to_lowercase());
                if !key.1.is_empty() && seen_lines.insert(key) {
                    combined_lines.push((rx, text));
                }
            }
        }
    }

    // Fallback Pass 3: Raw un-binarized image if binarized passes yielded sparse results
    if combined_lines.len() < 3 {
        if let Ok(lines) = recognize_text_lines(bgra_bytes, width, height).await {
            for (rx, text) in lines {
                let col_bucket = if rx < 0.36 { 0 } else if rx < 0.65 { 1 } else { 2 };
                let key = (col_bucket, text.trim().to_lowercase());
                if !key.1.is_empty() && seen_lines.insert(key) {
                    combined_lines.push((rx, text));
                }
            }
        }
    }

    Ok(combined_lines)
}

pub fn match_ocr_lines_to_augments(
    lines: &[(f32, String)],
    all_augments: &[MayhemAugmentItem],
) -> Vec<(usize, MayhemAugmentItem)> {
    use crate::services::dual_matcher::{normalize_token, DualSignalMatcher};

    let mut columns: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];

    for (rel_x, text) in lines {
        let trimmed = text.trim();
        if trimmed.is_empty() || is_category_badge_noise(trimmed) {
            continue;
        }

        let col = if *rel_x < 0.36 {
            0
        } else if *rel_x < 0.65 {
            1
        } else {
            2
        };
        columns[col].push(trimmed.to_string());
    }

    let matcher = DualSignalMatcher::default();
    let mut matches = Vec::new();

    for (col_idx, col_lines) in columns.iter().enumerate() {
        if col_lines.is_empty() {
            continue;
        }

        let col_joined = col_lines.join(" ");
        let col_text = normalize_token(&col_joined);
        let col_words: Vec<String> = col_joined
            .split_whitespace()
            .map(normalize_token)
            .filter(|w| !w.is_empty())
            .collect();

        let mut best_score = 0.0;
        let mut best_match: Option<MayhemAugmentItem> = None;

        for aug in all_augments {
            let (total_score, _title_sc, _desc_sc) =
                matcher.score_candidate(col_lines, &col_text, &col_words, aug);

            if total_score >= 0.55 && total_score > best_score {
                best_score = total_score;
                best_match = Some(aug.clone());
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

        // Crop to the 3-card region (x: 15%..85%, y: 18%..72%) if a full-screen image is passed
        let (crop_x, crop_y, crop_w, crop_h) = if width > 800 && height > 500 {
            (
                (0.15 * width as f64) as u32,
                (0.18 * height as f64) as u32,
                (0.70 * width as f64) as u32,
                (0.54 * height as f64) as u32,
            )
        } else {
            (0, 0, width, height)
        };

        let mut bgra = Vec::with_capacity((crop_w * crop_h * 4) as usize);
        for y in crop_y..(crop_y + crop_h) {
            for x in crop_x..(crop_x + crop_w) {
                let pixel = img.get_pixel(x, y);
                bgra.push(pixel[2]); // B
                bgra.push(pixel[1]); // G
                bgra.push(pixel[0]); // R
                bgra.push(pixel[3]); // A
            }
        }

        let lines = recognize_text_lines_multi_pass(&bgra, crop_w, crop_h).await?;
        let resp = crate::services::history::get_mayhem_augments(MayhemAugmentsQuery {
            champion_id,
            page_size: Some(1000),
            ..Default::default()
        })
        .await?;

        let matches = match_ocr_lines_to_augments(&lines, &resp.items);
        if matches.is_empty() {
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
                return Err("League client window not found or minimized. Ensure League is running.".to_string());
            }

            let fg = GetForegroundWindow();
            if fg != hwnd {
                return Err("League client is not in foreground. Please focus League of Legends window.".to_string());
            }

            let mut rc = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetClientRect(hwnd, &mut rc) == 0 {
                return Err("Failed to query League client window dimensions.".to_string());
            }

            let width = rc.right - rc.left;
            let height = rc.bottom - rc.top;
            if width <= 100 || height <= 100 {
                return Err("League client window size is invalid or minimized.".to_string());
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

        // Crop 3-card area (x: 15%..85%, y: 18%..72%) covering titles and descriptions while excluding HUD/top-bar
        let strip_x = rect.x + (rect.width as f64 * 0.15) as i32;
        let strip_y = rect.y + (rect.height as f64 * 0.18) as i32;
        let strip_w = (rect.width as f64 * 0.70) as i32;
        let strip_h = (rect.height as f64 * 0.54) as i32;

        let (bgra, bw, bh) = capture_bgra_rect(strip_x, strip_y, strip_w, strip_h)?;
        let lines = recognize_text_lines_multi_pass(&bgra, bw, bh).await?;
        if lines.is_empty() {
            return Err("No text recognized in augment selection area. Ensure augment selection is open.".to_string());
        }

        let resp = crate::services::history::get_mayhem_augments(MayhemAugmentsQuery {
            champion_id,
            page_size: Some(1000),
            with_augment_ids: Some(with_augment_ids),
            ..Default::default()
        })
        .await?;

        let matches = match_ocr_lines_to_augments(&lines, &resp.items);
        if matches.is_empty() {
            return Err("No augment cards found on screen. Ensure League is in foreground and augment selection is open.".to_string());
        }

        Ok(crate::services::history::enrich_overlay_augment_cards(champion_id, matches).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_and_fuzzy_similarity() {
        assert_eq!(levenshtein_distance("Mystic Punch", "Mystic Punch"), 0);
        assert_eq!(levenshtein_distance("Clown Colege", "Clown College"), 1);
        assert!(fuzzy_string_similarity("Clown Colege", "Clown College") >= 0.90);
        assert!(fuzzy_string_similarity("Symphony of War", "Symphony of Wor") >= 0.90);
    }

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
                pick_rate: 0.1,
                global_games: 100,
                global_win_rate: 0.5,
                global_wpa: 0.02,
                global_pick_rate: 0.1,
                aliases: vec!["Phenomenal Evil".into(), "Phenomenal Evil".into()]
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
                pick_rate: 0.1,
                global_games: 100,
                global_win_rate: 0.5,
                global_wpa: 0.05,
                global_pick_rate: 0.1,
                aliases: vec!["Eureka".into(), "Eureka".into()],
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

        // Crop title strip: rx = 0.21, ry = 0.36, rw = 0.58, rh = 0.12
        let strip_x = (width as f64 * 0.21) as u32;
        let strip_y = (height as f64 * 0.36) as u32;
        let strip_w = (width as f64 * 0.58) as u32;
        let strip_h = (height as f64 * 0.12) as u32;

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
