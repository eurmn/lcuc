use crate::models::history::{MayhemAugmentsQuery, OverlayAugmentCard};
use crate::services::native_overlay::OverlayPngSprite;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub static OVERLAY_ENABLED: AtomicBool = AtomicBool::new(true);
pub static OVERLAY_HAS_CARDS: AtomicBool = AtomicBool::new(false);
pub static OVERLAY_SCANNING: AtomicBool = AtomicBool::new(false);
static OVERLAY_LEVEL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static OVERLAY_GOLD: Mutex<f64> = Mutex::new(0.0);
static F8_WAS_DOWN: AtomicBool = AtomicBool::new(false);
static F7_WAS_DOWN: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    static ref OVERLAY_CHAMPION_NAME: Mutex<String> = Mutex::new(String::new());
    static ref OVERLAY_STRATEGY_LABEL: Mutex<String> = Mutex::new(String::new());
    static ref OVERLAY_OFFERED_CARDS: Mutex<Vec<OverlayAugmentCard>> = Mutex::new(Vec::new());
    pub static ref OVERLAY_RECOMMENDED_SPRITES: Mutex<Vec<OverlayPngSprite>> = Mutex::new(Vec::new());
    /// Monotonic ticket so a slow, older engine run cannot overwrite a newer
    /// panel after an item change lands mid-computation.
    static ref BUILD_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    /// The three-row build plan (next picks, full build, alternatives) as the
    /// last engine run produced it.
    static ref OVERLAY_BUILD_PANEL: Mutex<crate::services::overlay_presenter::OverlayBuildPanel> =
        Mutex::new(crate::services::overlay_presenter::OverlayBuildPanel::default());
    static ref OVERLAY_STATUS_TOAST: Mutex<Option<(String, bool, std::time::Instant)>> = Mutex::new(None);
}

/// Keys the in-game overlay reacts to. F8 rescans, F9/F10/F11 record which card
/// was picked (LEFT/MID/RIGHT), F6 undoes a wrong pick, F7 clears.
#[derive(Debug, Clone, Copy, Default)]
pub struct OverlayHotkeys {
    pub scan: bool,
    pub clear: bool,
    pub add_left: bool,
    pub add_mid: bool,
    pub add_right: bool,
    pub undo: bool,
}

impl OverlayHotkeys {
    pub fn picked_slot(&self) -> Option<usize> {
        match (self.add_left, self.add_mid, self.add_right) {
            (true, false, false) => Some(0),
            (false, true, false) => Some(1),
            (false, false, true) => Some(2),
            _ => None,
        }
    }
}

static F6_WAS_DOWN: AtomicBool = AtomicBool::new(false);
static F9_WAS_DOWN: AtomicBool = AtomicBool::new(false);
static F10_WAS_DOWN: AtomicBool = AtomicBool::new(false);
static F11_WAS_DOWN: AtomicBool = AtomicBool::new(false);

pub fn poll_overlay_hotkeys() -> OverlayHotkeys {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let down = |vk: i32, prev: &AtomicBool| -> bool {
            let is_down = (unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000) != 0;
            let was = prev.swap(is_down, Ordering::Relaxed);
            is_down && !was
        };

        OverlayHotkeys {
            scan: down(0x77, &F8_WAS_DOWN),
            clear: down(0x76, &F7_WAS_DOWN),
            undo: down(0x75, &F6_WAS_DOWN),
            add_left: down(0x78, &F9_WAS_DOWN),
            add_mid: down(0x79, &F10_WAS_DOWN),
            add_right: down(0x7A, &F11_WAS_DOWN),
        }
    }
    #[cfg(not(windows))]
    {
        OverlayHotkeys::default()
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

pub fn set_overlay_vitals(level: u32, gold: f64) {
    let previous_level = OVERLAY_LEVEL.swap(level, Ordering::Relaxed);
    let previous_gold = OVERLAY_GOLD.lock().map(|g| *g).unwrap_or(0.0);
    if let Ok(mut g) = OVERLAY_GOLD.lock() {
        *g = gold;
    }
    if previous_level != level || (previous_gold - gold).abs() > 1.0 {
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
    let build_panel = OVERLAY_BUILD_PANEL
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
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

    let level = OVERLAY_LEVEL.load(Ordering::Relaxed);
    let gold = OVERLAY_GOLD.lock().map(|g| *g).unwrap_or(0.0);
    let rows = crate::services::overlay_presenter::build_rows(&cards, 26);
    let hud_text =
        crate::services::overlay_presenter::build_hud_text(&champ, level, gold, !cards.is_empty());

    let payload = crate::services::native_overlay::OverlayRenderPayload {
        visible: true,
        scanning,
        champion_name: champ,
        strategy_label: strat,
        cards,
        rows,
        build_panel,
        hud_text,
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

/// Item icons for the build panel. Sprites are keyed by item id, so an item
/// that appears in both "next" and the full build decodes once.
pub async fn build_panel_item_sprites(item_ids: &[u32]) -> Vec<OverlayPngSprite> {
    let mut unique: Vec<u32> = Vec::new();
    for &id in item_ids {
        if id > 0 && !unique.contains(&id) {
            unique.push(id);
        }
    }
    if unique.is_empty() {
        return Vec::new();
    }

    let cache_dir = crate::services::asset_cache::get_asset_cache_dir().join("items");
    let _ = tokio::fs::create_dir_all(&cache_dir).await;

    let items_catalog = crate::services::history::get_items_by_id().await.ok();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let mut sprites = Vec::with_capacity(unique.len());

    for item_id in unique {
        let item_path = cache_dir.join(format!("{item_id}.png"));
        let exists = match tokio::fs::metadata(&item_path).await {
            Ok(m) => m.len() > 0,
            Err(_) => false,
        };

        if !exists {
            // Prefer the catalog's official icon url, fall back to the direct
            // communitydragon path (some ids only resolve there).
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
            && std::fs::metadata(&item_path)
                .map(|m| m.len() > 0)
                .unwrap_or(false)
        {
            Some(item_path.to_string_lossy().to_string())
        } else {
            None
        };
        let png_bytes = if png_path.is_some() {
            tokio::fs::read(&item_path).await.ok()
        } else {
            None
        };

        sprites.push(OverlayPngSprite {
            id: crate::services::native_overlay::build_item_sprite_id(item_id),
            png_path,
            png_base64_or_bytes: png_bytes,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        });
    }

    sprites
}

fn item_display_name(
    names: &std::collections::HashMap<i32, crate::services::history::CachedItemInfo>,
    item_id: u32,
) -> String {
    names
        .get(&(item_id as i32))
        .map(|i| i.name.clone())
        .unwrap_or_else(|| format!("Item {item_id}"))
}

fn build_item_view(
    dto: &crate::agent::mayhem_engine::ItemRecommendationDto,
    names: &std::collections::HashMap<i32, crate::services::history::CachedItemInfo>,
) -> crate::services::overlay_presenter::OverlayBuildItemView {
    // The engine only knows ids for items and may carry a raw id for an
    // augment, so both partners are named here from the catalogs.
    let reason = if dto.synergy_partner_id > 0 {
        let partner = if dto.synergy_partner_is_augment {
            crate::services::history::aram_augment_name_for(dto.synergy_partner_id)
                .unwrap_or_else(|| dto.reason.clone())
        } else {
            item_display_name(names, dto.synergy_partner_id)
        };
        format!(
            "Pairs with {partner} (+{:.1}% synergy)",
            dto.synergy_partner_lift * 100.0
        )
    } else {
        dto.reason.clone()
    };

    crate::services::overlay_presenter::OverlayBuildItemView {
        item_id: dto.item_id,
        name: item_display_name(names, dto.item_id),
        win_rate: dto.win_rate,
        lift: dto.lift,
        sample_size: dto.sample_size,
        reason,
    }
}

/// Turns one engine run into the overlay's three rows plus their icon sprites.
pub async fn apply_build_recommendation(
    rec: &crate::agent::mayhem_engine::MayhemBuildRecommendationResponse,
) {
    let ticket = BUILD_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let names = crate::services::history::get_items_by_id()
        .await
        .unwrap_or_default();

    let next: Vec<_> = rec
        .next_items
        .iter()
        .map(|d| build_item_view(d, &names))
        .collect();
    let full: Vec<_> = rec
        .completed_build
        .iter()
        .map(|d| build_item_view(d, &names))
        .collect();
    let alts: Vec<_> = rec
        .alternative_items
        .iter()
        .filter(|d| d.sample_size > 0)
        .map(|d| build_item_view(d, &names))
        .collect();

    let panel = crate::services::overlay_presenter::build_build_panel(
        &next,
        &full,
        &alts,
        rec.build_winrate,
        rec.stats.match_count,
        26,
    );

    let ids: Vec<u32> = next
        .iter()
        .chain(full.iter())
        .chain(alts.iter())
        .map(|v| v.item_id)
        .collect();
    let sprites = build_panel_item_sprites(&ids).await;

    if BUILD_SEQ.load(Ordering::Relaxed) != ticket {
        return; // a newer run is already in flight
    }
    publish_build_panel(panel, sprites);
}

/// Replaces the panel and its icons, then repaints.
fn publish_build_panel(
    panel: crate::services::overlay_presenter::OverlayBuildPanel,
    sprites: Vec<OverlayPngSprite>,
) {
    if let Ok(mut lock) = OVERLAY_BUILD_PANEL.lock() {
        *lock = panel;
    }
    if let Ok(mut lock) = OVERLAY_RECOMMENDED_SPRITES.lock() {
        *lock = sprites;
    }
    refresh_native_overlay();
}

/// Fallback for payloads that only carry item ids (no engine result): names in
/// the next/full rows, without invented statistics.
async fn apply_build_ids(boots: Option<u32>, next_items: Vec<u32>, completed_build: Vec<u32>) {
    let names = crate::services::history::get_items_by_id()
        .await
        .unwrap_or_default();
    let view = |id: u32| crate::services::overlay_presenter::OverlayBuildItemView {
        item_id: id,
        name: item_display_name(&names, id),
        win_rate: 0.0,
        lift: 0.0,
        sample_size: 0,
        reason: String::new(),
    };

    let mut next: Vec<_> = next_items.into_iter().filter(|&id| id > 0).map(view).collect();
    if let Some(boot) = boots.filter(|b| *b > 0) {
        if !next.iter().any(|v| v.item_id == boot) {
            next.insert(0, view(boot));
        }
    }
    let full: Vec<_> = completed_build
        .into_iter()
        .filter(|&id| id > 0)
        .map(view)
        .collect();

    let panel = crate::services::overlay_presenter::build_build_panel(&next, &full, &[], 0.0, 0, 26);
    let ids: Vec<u32> = next
        .iter()
        .chain(full.iter())
        .map(|v| v.item_id)
        .collect();
    let sprites = build_panel_item_sprites(&ids).await;
    publish_build_panel(panel, sprites);
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

    // Preferred path: the app UI computed with the same engine, so it sends the
    // whole result — reasons, sample sizes and alternatives included.
    if let Some(rec_val) = payload.get("buildEngineRec").filter(|v| !v.is_null()) {
        if let Ok(rec) = serde_json::from_value::<
            crate::agent::mayhem_engine::MayhemBuildRecommendationResponse,
        >(rec_val.clone())
        {
            tauri::async_runtime::spawn(async move {
                apply_build_recommendation(&rec).await;
            });
            return;
        }
    }

    if has_item_keys {
        tauri::async_runtime::spawn(async move {
            apply_build_ids(boots, next_items, completed_build).await;
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

// ---------------------------------------------------------------
// Augment scanning
// ---------------------------------------------------------------
//
// Identification lives in `services::augment_scan`: card art fingerprints pick
// the augment, and only ambiguous art falls back to a tiny title-strip OCR.
// Everything expensive (winrate projection, synergy index, verdicts) runs after
// the cards are already on screen, so F8 never stalls the game polling loop.

/// Screen region that can contain the three cards, as fractions of the game
/// window. Wide enough for any 16:9 layout, small enough to stay cheap.
const SCAN_X0: f64 = 0.14;
const SCAN_Y0: f64 = 0.10;
const SCAN_W: f64 = 0.72;
const SCAN_H: f64 = 0.62;

lazy_static::lazy_static! {
    static ref OVERLAY_APP_HANDLE: Mutex<Option<tauri::AppHandle>> = Mutex::new(None);
    static ref SCAN_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
    /// Augments picked this game, in pick order. These drive every later scan's
    /// recommendation and the item build, so they must outlive a card scan.
    static ref IN_GAME_AUGMENTS: Mutex<Vec<OwnedAugment>> = Mutex::new(Vec::new());
    static ref IN_GAME_CHAMPION_ID: Mutex<Option<i64>> = Mutex::new(None);
    /// Fingerprint of the inventory the current build was computed for.
    static ref INVENTORY_FINGERPRINT: Mutex<u64> = Mutex::new(0);
}

/// An augment the player owns this game.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnedAugment {
    pub id: String,
    pub name: String,
    pub slot: usize,
}

pub fn owned_augments() -> Vec<OwnedAugment> {
    IN_GAME_AUGMENTS
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

pub fn owned_augment_ids() -> Vec<String> {
    owned_augments().into_iter().map(|a| a.id).collect()
}

pub fn remember_champion_id(champion_id: Option<i64>) {
    if let Ok(mut slot) = IN_GAME_CHAMPION_ID.lock() {
        if champion_id.is_some() {
            *slot = champion_id;
        }
    }
}

fn remembered_champion_id() -> Option<i64> {
    IN_GAME_CHAMPION_ID.lock().ok().and_then(|g| *g)
}

/// Records a picked augment and refreshes everything that depends on it: the
/// verdicts of the cards still on screen, the item build, and the app UI.
/// Returns false when the augment was already owned.
pub fn add_owned_augment(id: &str, name: &str, slot: usize) -> bool {
    let id = id.trim();
    if id.is_empty() {
        return false;
    }
    let augmented = {
        let Ok(mut list) = IN_GAME_AUGMENTS.lock() else {
            return false;
        };
        if list.iter().any(|a| a.id == id) {
            false
        } else {
            list.push(OwnedAugment {
                id: id.to_string(),
                name: name.trim().to_string(),
                slot,
            });
            true
        }
    };
    if !augmented {
        return false;
    }
    let owned = owned_augments();
    crate::logger::write(
        crate::logger::Level::Info,
        "live_overlay",
        &format!(
            "picked augment #{} '{}' (now {} owned)",
            slot + 1,
            name,
            owned.len()
        ),
        None,
    );
    show_overlay_status_toast(&format!("Added {name}"), false);
    refresh_after_owned_change();
    true
}

/// Records the card in `slot` as picked. Returns the augment, or None when no
/// cards are on screen (nothing to pick from).
pub fn record_pick_for_slot(slot: usize) -> Option<OwnedAugment> {
    let card = {
        let cards = OVERLAY_OFFERED_CARDS.lock().ok()?;
        cards.iter().find(|c| c.slot == slot).cloned()?
    };
    if add_owned_augment(&card.id, &card.name, slot) {
        owned_augments().last().cloned()
    } else {
        Some(OwnedAugment {
            id: card.id,
            name: card.name,
            slot,
        })
    }
}

/// Removes the most recently added augment (mis-detected pick).
pub fn undo_owned_augment() -> Option<OwnedAugment> {
    let removed = IN_GAME_AUGMENTS.lock().ok().and_then(|mut l| l.pop());
    if removed.is_some() {
        refresh_after_owned_change();
    }
    removed
}

pub fn clear_owned_augments() {
    let had_any = IN_GAME_AUGMENTS
        .lock()
        .map(|mut l| !l.is_empty() && {
            l.clear();
            true
        })
        .unwrap_or(false);
    if had_any {
        refresh_after_owned_change();
    }
}

/// Forgets picks for a finished game, without recomputing anything.
pub fn reset_owned_augments() {
    if let Ok(mut list) = IN_GAME_AUGMENTS.lock() {
        list.clear();
    }
    if let Ok(mut champ) = IN_GAME_CHAMPION_ID.lock() {
        *champ = None;
    }
}

fn refresh_after_owned_change() {
    let owned = owned_augments();
    let champion_id = remembered_champion_id();
    emit_owned_augments(&owned);

    let cards = OVERLAY_OFFERED_CARDS
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    if !cards.is_empty() {
        spawn_overlay_card_enrichment(champion_id, owned_augment_ids(), cards);
    }
    tauri::async_runtime::spawn(async move {
        if let Err(err) = recompute_in_game_build(champion_id).await {
            crate::logger::write(
                crate::logger::Level::Warn,
                "live_overlay",
                &format!("build recompute after pick failed: {err}"),
                None,
            );
        }
    });
}

/// Pushes the owned list to the app UI so its chips and its own build recompute
/// agree with the overlay.
fn emit_owned_augments(owned: &[OwnedAugment]) {
    use tauri::Emitter;
    let Ok(slot) = OVERLAY_APP_HANDLE.lock() else {
        return;
    };
    let Some(app) = slot.as_ref() else {
        return;
    };
    let names: Vec<String> = owned.iter().map(|a| a.name.clone()).collect();
    let _ = app.emit(
        "overlay:augments-changed",
        serde_json::json!({
            "augments": owned,
            "augmentIds": names,
            "count": owned.len(),
        }),
    );
    // The champ-select view merges this into its own chip list.
    let _ = app.emit(
        "overlay:state-sync",
        serde_json::json!({ "source": "overlay_pick", "augmentIds": names }),
    );
}

/// Recomputes the recommended item build from the augments actually owned and
/// pushes it straight to the native overlay, so a pick changes the build even
/// when no app window is being looked at.
async fn recompute_in_game_build(champion_id: Option<i64>) -> Result<(), String> {
    let champ = champion_id.filter(|c| *c > 0).unwrap_or(0) as u32;
    if champ == 0 {
        return Ok(());
    }
    let owned = owned_augment_ids();
    let offered: Vec<String> = OVERLAY_OFFERED_CARDS
        .lock()
        .map(|cards| cards.iter().map(|c| c.id.clone()).collect())
        .unwrap_or_default();

    let inventory: Vec<u32> = crate::services::live_game::fetch_live_game_state()
        .await
        .map(|state| state.items.iter().map(|i| i.item_id as u32).collect())
        .unwrap_or_default();

    let rec = crate::commands::build_engine::compute_mayhem_build_recommendation(
        Some(champ),                                          // champion_id
        None,                                                 // championId
        Some(owned),                                          // locked_augments
        None,                                                 // lockedAugments
        None,                                                 // owned_augments
        None,                                                 // ownedAugments
        Some(inventory),                                      // locked_items
        None,                                                 // lockedItems
        None,                                                 // owned_items
        None,                                                 // ownedItems
        if offered.is_empty() { None } else { Some(offered) }, // offered_cards
        None,                                                 // offeredCards
        None,                                                 // seen_augments
        None,                                                 // seenAugments
        None,                                                 // augment_slot
        None,                                                 // augmentSlot
        None,                                                 // command_line_reroll
        None,                                                 // commandLineReroll
    )
    .await?;

    apply_build_recommendation(&rec).await;
    Ok(())
}

/// Called by the live-game poller when the inventory changes (or the match
/// starts), so the three rows follow items as well as augments.
pub fn refresh_in_game_build() {
    let champion_id = remembered_champion_id();
    let owned = owned_augment_ids();
    let inventory_size = INVENTORY_FINGERPRINT.lock().map(|f| *f).unwrap_or(0);
    crate::logger::write(
        crate::logger::Level::Debug,
        "live_overlay",
        &format!(
            "build recompute: champion={champion_id:?}, augments={}, inventory={inventory_size}",
            owned.len()
        ),
        None,
    );
    tauri::async_runtime::spawn(async move {
        if let Err(err) = recompute_in_game_build(champion_id).await {
            crate::logger::write(
                crate::logger::Level::Warn,
                "live_overlay",
                &format!("in-game build recompute failed: {err}"),
                None,
            );
        }
    });
}

/// Sorted item ids the build was last computed for. The poller compares against
/// this so it only recomputes when something was actually bought or sold.
pub fn inventory_fingerprint(items: &[i64]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut sorted: Vec<i64> = items.iter().copied().filter(|id| *id > 0).collect();
    sorted.sort_unstable();
    sorted.dedup();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    sorted.hash(&mut hasher);
    hasher.finish()
}

pub fn note_inventory_fingerprint(fingerprint: u64) -> bool {
    let Ok(mut slot) = INVENTORY_FINGERPRINT.lock() else {
        return false;
    };
    if *slot == fingerprint {
        return false;
    }
    *slot = fingerprint;
    true
}

pub fn reset_inventory_fingerprint() {
    if let Ok(mut slot) = INVENTORY_FINGERPRINT.lock() {
        *slot = 0;
    }
}

/// Geometry-only probe of the augment panel: is it on screen, and where.
/// Cheap enough to run a few times a second; no fingerprints, no OCR.
#[cfg(windows)]
pub fn probe_augment_panel() -> Option<crate::services::augment_scan::PanelProbe> {
    let rect = league_scan_rect().ok()?;
    if !rect.is_foreground {
        return None;
    }
    let frame = capture_scan_frame(&rect).ok()?;
    crate::services::augment_scan::probe_panel(frame)
}


/// The overlay needs an app handle to push background results to the frontend.
pub fn set_overlay_app_handle(app: tauri::AppHandle) {
    if let Ok(mut slot) = OVERLAY_APP_HANDLE.lock() {
        *slot = Some(app);
    }
}

fn emit_cards_updated(cards: &[OverlayAugmentCard]) {
    use tauri::Emitter;
    if let Ok(slot) = OVERLAY_APP_HANDLE.lock() {
        if let Some(app) = slot.as_ref() {
            let _ = app.emit("overlay:cards-updated", cards);
        }
    }
}

pub fn overlay_has_cards() -> bool {
    OVERLAY_HAS_CARDS.load(Ordering::Relaxed)
}

fn begin_scan() -> bool {
    !SCAN_IN_FLIGHT.swap(true, Ordering::SeqCst)
}

fn end_scan() {
    SCAN_IN_FLIGHT.store(false, Ordering::SeqCst);
}

/// True while a scan is running, so hotkey handlers can ignore repeat presses.
pub fn scan_in_flight() -> bool {
    SCAN_IN_FLIGHT.load(Ordering::SeqCst)
}

/// Capture just the band that can hold the three cards.
#[cfg(windows)]
fn capture_scan_frame(
    rect: &GameWindowRect,
) -> Result<crate::services::augment_scan::Frame, String> {
    let x = rect.x + (rect.width as f64 * SCAN_X0) as i32;
    let y = rect.y + (rect.height as f64 * SCAN_Y0) as i32;
    let w = ((rect.width as f64 * SCAN_W) as i32).max(1);
    let h = ((rect.height as f64 * SCAN_H) as i32).max(1);
    let (buf, bw, bh) = capture_bgra_rect(x, y, w, h)?;
    Ok(crate::services::augment_scan::Frame::from_bgra_capture(
        buf, bw, bh,
    ))
}

/// Window rect of the running game, requiring it to be the foreground window.
#[cfg(windows)]
fn league_scan_rect() -> Result<GameWindowRect, String> {
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

    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), title_wide.as_ptr());
        if hwnd.is_null() || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return Err("League of Legends window not found or minimised.".to_string());
        }
        if GetForegroundWindow() != hwnd {
            return Err("Focus League of Legends, then press F8.".to_string());
        }

        let mut rc = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetClientRect(hwnd, &mut rc) == 0 {
            return Err("Failed to query the League window size.".to_string());
        }
        let width = rc.right - rc.left;
        let height = rc.bottom - rc.top;
        if width <= 100 || height <= 100 {
            return Err("League window size is invalid or minimised.".to_string());
        }

        let mut pt = POINT { x: 0, y: 0 };
        ClientToScreen(hwnd, &mut pt);

        Ok(GameWindowRect {
            found: true,
            is_foreground: true,
            x: pt.x,
            y: pt.y,
            width,
            height,
        })
    }
}

/// Identifies the offered augment cards. Returns as soon as the names are known;
/// winrates and verdicts arrive later via `spawn_overlay_card_enrichment`.
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
        if !begin_scan() {
            return Err("A scan is already running.".to_string());
        }
        let result = scan_in_game_augment_cards_inner(champion_id, with_augment_ids).await;
        end_scan();
        result
    }
}

#[cfg(windows)]
async fn scan_in_game_augment_cards_inner(
    champion_id: Option<i64>,
    with_augment_ids: Vec<String>,
) -> Result<Vec<OverlayAugmentCard>, String> {
    remember_champion_id(champion_id);

    // Cards animate in, so a capture taken too early yields half a panel. Retry
    // instead of making the player press F8 again.
    let scanner = crate::services::augment_scan::scanner()
        .await
        .map_err(|e| e.to_string())?;

    let mut last_error: Option<String> = None;
    let mut identified = Vec::new();
    for attempt in 0..SCAN_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(SCAN_RETRY_MS)).await;
        }
        let rect = league_scan_rect()?;
        let frame = capture_scan_frame(&rect)?;
        match scanner.identify(&frame).await {
            Ok(cards) if !cards.is_empty() => {
                let complete = cards.len() >= 3;
                identified = cards;
                if complete {
                    break;
                }
            }
            Ok(_) => last_error = Some("No augment cards found on screen.".to_string()),
            Err(err) => last_error = Some(err.to_string()),
        }
    }

    if identified.is_empty() {
        return Err(last_error.unwrap_or_else(|| "No augment cards found on screen.".to_string()));
    }

    // The pool key is the id the mode's own statistics are keyed by. Resolving it
    // by name picked the wrong numeric id for augments that exist in both Arena
    // and ARAM (Jeweled Gauntlet was the visible one: 0 games, always).
    let mut canonical: Option<std::collections::HashMap<String, String>> = None;
    let mut cards: Vec<OverlayAugmentCard> = Vec::with_capacity(identified.len());
    for id in &identified {
        let key = id.entry.key.trim();
        let name_key = id.entry.name.trim().to_lowercase();
        let resolved = if key.is_empty() || key.chars().all(|c| !c.is_ascii_digit()) {
            let map = match canonical.as_ref() {
                Some(map) => map,
                None => {
                    let names: Vec<String> =
                        identified.iter().map(|c| c.entry.name.clone()).collect();
                    let map = crate::services::history::resolve_canonical_augment_ids(&names).await;
                    canonical = Some(map);
                    canonical.as_ref().unwrap()
                }
            };
            map.get(&name_key).cloned().unwrap_or_else(|| key.to_string())
        } else {
            key.to_string()
        };
        cards.push(OverlayAugmentCard {
            slot: id.slot,
            id: resolved,
            name: id.entry.name.clone(),
            rarity: id.entry.rarity.clone(),
            pending: true,
            unconfirmed: id.ambiguous,
            ..Default::default()
        });
    }

    // Cards already visible: show them immediately, then fill in the numbers.
    update_overlay_cards(&cards);
    let _ = emit_cards_updated_result(&cards);

    let mut locked = with_augment_ids;
    for id in owned_augment_ids() {
        if !locked.contains(&id) {
            locked.push(id);
        }
    }
    spawn_overlay_card_enrichment(champion_id, locked, cards.clone());
    Ok(cards)
}

/// Number of captures tried before giving up on a card scan.
const SCAN_ATTEMPTS: usize = 3;
/// Delay between those captures.
const SCAN_RETRY_MS: u64 = 130;

fn emit_cards_updated_result(cards: &[OverlayAugmentCard]) -> Result<(), String> {
    use tauri::Emitter;
    if let Ok(slot) = OVERLAY_APP_HANDLE.lock() {
        if let Some(app) = slot.as_ref() {
            return app
                .emit("overlay:cards-updated", cards)
                .map_err(|e| e.to_string());
        }
    }
    Ok(())
}

/// Fills in winrates and verdicts after the cards are already visible.
pub fn spawn_overlay_card_enrichment(
    champion_id: Option<i64>,
    locked_augments: Vec<String>,
    base: Vec<OverlayAugmentCard>,
) {
    if base.is_empty() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let started = std::time::Instant::now();
        let mut cards = base;

        // Global projection only: the champion-scoped recompute costs tens of
        // seconds and is not needed to price an offered card.
        if let Ok(resp) = crate::services::history::get_mayhem_augments(MayhemAugmentsQuery {
            page_size: Some(1000),
            ..Default::default()
        })
        .await
        {
            let mut by_id = std::collections::HashMap::new();
            let mut by_name = std::collections::HashMap::new();
            for item in resp.items {
                by_name.insert(item.name.trim().to_lowercase(), item.clone());
                by_id.insert(item.id.clone(), item);
            }
            for card in cards.iter_mut() {
                // Identity by id first, then by display name: ids drift between
                // patches and between the Arena and ARAM variants of an augment.
                let item = by_id
                    .get(&card.id)
                    .or_else(|| by_name.get(&card.name.trim().to_lowercase()));
                if let Some(item) = item {
                    card.wpa = item.wpa;
                    card.win_rate = if item.win_rate > 0.0 {
                        item.win_rate
                    } else {
                        item.global_win_rate
                    };
                    card.games = if item.games > 0 {
                        item.games
                    } else {
                        item.global_games
                    };
                }
            }
        }

        let inventory = crate::services::live_game::fetch_live_game_state()
            .await
            .map(|state| {
                state
                    .items
                    .iter()
                    .map(|i| i.item_id as u32)
                    .collect::<Vec<u32>>()
            })
            .unwrap_or_default();
        apply_card_decisions(champion_id, &locked_augments, &inventory, &mut cards).await;

        // Whatever happened above, the cards are no longer "reading stats".
        for card in cards.iter_mut() {
            card.pending = false;
        }

        update_overlay_cards(&cards);
        emit_cards_updated(&cards);
        crate::logger::write(
            crate::logger::Level::Info,
            "live_overlay",
            &format!(
                "card enrichment finished in {}ms ({} cards)",
                started.elapsed().as_millis(),
                cards.len()
            ),
            None,
        );
    });
}

async fn apply_card_decisions(
    champion_id: Option<i64>,
    locked_augments: &[String],
    inventory_item_ids: &[u32],
    cards: &mut [OverlayAugmentCard],
) {
    let Some(champ_id) = champion_id.filter(|id| *id > 0) else {
        return;
    };
    if cards.is_empty() {
        return;
    }
    if crate::commands::build_engine::ensure_index_ready()
        .await
        .is_err()
    {
        return;
    }

    let tier = crate::agent::mayhem_engine::get_augment_meta(&cards[0].id).tier;
    let offered: Vec<u32> = cards
        .iter()
        .map(|c| crate::agent::mayhem_engine::augment_numeric_id(&c.id))
        .collect();
    let locked: Vec<u32> = locked_augments
        .iter()
        .map(|s| crate::agent::mayhem_engine::augment_numeric_id(s))
        .collect();

    let decisions = {
        let Ok(read) = crate::agent::mayhem_engine::MAYHEM_SYNERGY_INDEX.read() else {
            return;
        };
        let Some(index) = read.as_ref() else {
            return;
        };
        crate::agent::mayhem_engine::decide_offered_cards(
            index,
            champ_id as u16,
            &offered,
            &locked,
            inventory_item_ids,
            &tier,
            &[],
        )
    };

    for (card, decision) in cards.iter_mut().zip(decisions.iter()) {
        card.wpa = decision.wpa;
        card.win_rate = decision.win_rate;
        card.score = Some(decision.effect);
        card.decision = Some(decision.verdict.to_string());
        card.rank = Some(decision.rank);
        card.win_rate_low = Some(decision.win_rate_low);
        card.win_rate_high = Some(decision.win_rate_high);
        card.reroll_gain = Some(decision.reroll_gain);
        card.reroll_probability = Some(decision.reroll_probability);
        if card.games == 0 {
            card.games = decision.games as usize;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_frame_from_capture_forces_opaque_alpha() {
        // GDI leaves the fourth byte at zero; WinRT OCR treats that as fully
        // transparent, so the frame constructor must repair it.
        let raw = vec![10u8, 20, 30, 0, 40, 50, 60, 0];
        let frame = crate::services::augment_scan::Frame::from_bgra_capture(raw, 2, 1);
        assert_eq!(frame.bgra[3], 255);
        assert_eq!(frame.bgra[7], 255);
        assert_eq!(frame.bgra[0], 10);
    }

    /// The reported bug: picking an augment changed nothing. This walks the same
    /// path a pick takes and requires the recommendation (and the item list) to move.
    #[tokio::test]
    async fn a_picked_augment_changes_the_build() {
        let db = dirs::data_local_dir()
            .map(|d| d.join("lcu-companion").join("global-aram.db"))
            .unwrap_or_default();
        if !db.exists() {
            return; // no statistics database on this machine
        }

        let with = |augments: Vec<String>| {
            crate::commands::build_engine::compute_mayhem_build_recommendation(
                Some(222),
                None,
                Some(augments),
                None,
                None,
                None,
                Some(Vec::new()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        };

        let before = with(Vec::new()).await.expect("baseline build");
        let after = with(vec!["Jeweled Gauntlet".to_string()])
            .await
            .expect("build with the pick");

        let items_before: Vec<u32> = before.next_items.iter().map(|i| i.item_id).collect();
        let items_after: Vec<u32> = after.next_items.iter().map(|i| i.item_id).collect();
        assert!(
            (before.build_wpa - after.build_wpa).abs() > 1e-9 || items_before != items_after,
            "picking an augment must change the build (wpa {:.5} vs {:.5}, items {items_before:?} vs {items_after:?})",
            before.build_wpa,
            after.build_wpa
        );
    }

    #[test]
    fn scan_region_covers_the_card_area() {
        // Cards span roughly x 0.22..0.78 and y 0.17..0.66 of the window.
        assert!(SCAN_X0 < 0.22 && SCAN_X0 + SCAN_W > 0.78);
        assert!(SCAN_Y0 < 0.17 && SCAN_Y0 + SCAN_H > 0.66);
    }
}
