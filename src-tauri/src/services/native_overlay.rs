use crate::models::history::OverlayAugmentCard;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlayAnchorLayout {
    pub game_x: i32,
    pub game_y: i32,
    pub game_width: i32,
    pub game_height: i32,
    pub hud_banner_rect: [i32; 4],
    pub card_rects: [[i32; 4]; 3],
    pub title_strip_rect: [i32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlayPngSprite {
    pub id: String,
    pub png_path: Option<String>,
    pub png_base64_or_bytes: Option<Vec<u8>>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OverlayRenderPayload {
    pub visible: bool,
    pub scanning: bool,
    pub champion_name: String,
    pub strategy_label: String,
    pub cards: Vec<OverlayAugmentCard>,
    pub sprites: Vec<OverlayPngSprite>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum OverlayIpcCommand {
    CalibrateOnce {
        game_x: i32,
        game_y: i32,
        game_width: i32,
        game_height: i32,
    },
    Render {
        payload: OverlayRenderPayload,
    },
    Hide,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum OverlayIpcEvent {
    Ready { pid: u32 },
    Calibrated { layout: OverlayAnchorLayout },
    Error { message: String },
}

pub fn compute_default_anchor_layout(
    game_x: i32,
    game_y: i32,
    game_width: i32,
    game_height: i32,
) -> OverlayAnchorLayout {
    let gw = game_width as f64;
    let gh = game_height as f64;

    let banner_w = 340;
    let banner_h = 42;
    let banner_x = (game_width - banner_w - 24).max(0);
    let banner_y = 18;

    let strip_x = (gw * 0.21) as i32;
    let strip_y = (gh * 0.385) as i32;
    let strip_w = (gw * 0.58) as i32;
    let strip_h = (gh * 0.065) as i32;

    let card_w = (gw * 0.17) as i32;
    let card_h = (gh * 0.24) as i32;
    let card_y = (gh * 0.08) as i32;

    let c0_x = (gw * 0.21) as i32;
    let c1_x = (gw * 0.415) as i32;
    let c2_x = (gw * 0.62) as i32;

    OverlayAnchorLayout {
        game_x,
        game_y,
        game_width,
        game_height,
        hud_banner_rect: [banner_x, banner_y, banner_w, banner_h],
        card_rects: [
            [c0_x, card_y, card_w, card_h],
            [c1_x, card_y, card_w, card_h],
            [c2_x, card_y, card_w, card_h],
        ],
        title_strip_rect: [strip_x, strip_y, strip_w, strip_h],
    }
}

pub fn calibrate_layout_from_single_screenshot(
    game_x: i32,
    game_y: i32,
    game_width: i32,
    game_height: i32,
) -> OverlayAnchorLayout {
    let mut layout = compute_default_anchor_layout(game_x, game_y, game_width, game_height);
    #[cfg(windows)]
    {
        if let Ok((bgra, bw, bh)) =
            crate::services::live_overlay::capture_bgra_rect(game_x, game_y, game_width, game_height)
        {
            refine_card_columns_from_bgra(&bgra, bw, bh, &mut layout);
            drop(bgra);
        }
    }
    layout
}

#[cfg(windows)]
fn refine_card_columns_from_bgra(
    bgra: &[u8],
    width: u32,
    height: u32,
    layout: &mut OverlayAnchorLayout,
) {
    if width < 300 || height < 200 || (width * height * 4) as usize > bgra.len() {
        return;
    }

    let sample_y = (height as f64 * 0.40) as usize;
    if sample_y >= height as usize {
        return;
    }

    let default_col_centers = [
        (width as f64 * 0.295) as usize,
        (width as f64 * 0.500) as usize,
        (width as f64 * 0.705) as usize,
    ];

    let search_radius = (width as f64 * 0.05) as usize;

    for (slot_idx, &expected_center) in default_col_centers.iter().enumerate() {
        let x_start = expected_center.saturating_sub(search_radius);
        let x_end = (expected_center + search_radius).min((width - 2) as usize);
        if x_start >= x_end {
            continue;
        }

        let mut max_grad = 0i32;
        let mut best_x = expected_center;

        for x in x_start..x_end {
            let left_idx = ((sample_y * width as usize + x - 1) * 4) as usize;
            let right_idx = ((sample_y * width as usize + x + 1) * 4) as usize;

            let b_diff = (bgra[right_idx] as i32 - bgra[left_idx] as i32).abs();
            let g_diff = (bgra[right_idx + 1] as i32 - bgra[left_idx + 1] as i32).abs();
            let r_diff = (bgra[right_idx + 2] as i32 - bgra[left_idx + 2] as i32).abs();
            let grad = b_diff + g_diff + r_diff;

            if grad > max_grad {
                max_grad = grad;
                best_x = x;
            }
        }

        // Only refine if strong edge detected (> 150 gradient)
        if max_grad > 150 {
            let half_card = (layout.card_rects[slot_idx][2] / 2) as usize;
            let new_card_x = (best_x.saturating_sub(half_card)) as i32;
            layout.card_rects[slot_idx][0] = new_card_x;
        }
    }
}

pub fn fill_premultiplied_rect(
    buf: &mut [u8],
    buf_w: u32,
    buf_h: u32,
    rect: [i32; 4],
    r: u8,
    g: u8,
    b: u8,
    a: u8,
    radius: i32,
) {
    let [rx, ry, rw, rh] = rect;
    if rw <= 0 || rh <= 0 || a == 0 {
        return;
    }

    let min_x = rx.max(0);
    let max_x = (rx + rw).min(buf_w as i32);
    let min_y = ry.max(0);
    let max_y = (ry + rh).min(buf_h as i32);
    if min_x >= max_x || min_y >= max_y {
        return;
    }

    let premul_r = (r as u32 * a as u32) / 255;
    let premul_g = (g as u32 * a as u32) / 255;
    let premul_b = (b as u32 * a as u32) / 255;
    let inv_a = 255 - a as u32;

    let rad_sq = radius * radius;

    for y in min_y..max_y {
        let rel_y = y - ry;
        for x in min_x..max_x {
            let rel_x = x - rx;

            if radius > 0 {
                let in_top_left = rel_x < radius && rel_y < radius;
                let in_top_right = rel_x >= rw - radius && rel_y < radius;
                let in_bot_left = rel_x < radius && rel_y >= rh - radius;
                let in_bot_right = rel_x >= rw - radius && rel_y >= rh - radius;

                if in_top_left {
                    let dx = radius - 1 - rel_x;
                    let dy = radius - 1 - rel_y;
                    if dx * dx + dy * dy > rad_sq {
                        continue;
                    }
                } else if in_top_right {
                    let dx = rel_x - (rw - radius);
                    let dy = radius - 1 - rel_y;
                    if dx * dx + dy * dy > rad_sq {
                        continue;
                    }
                } else if in_bot_left {
                    let dx = radius - 1 - rel_x;
                    let dy = rel_y - (rh - radius);
                    if dx * dx + dy * dy > rad_sq {
                        continue;
                    }
                } else if in_bot_right {
                    let dx = rel_x - (rw - radius);
                    let dy = rel_y - (rh - radius);
                    if dx * dx + dy * dy > rad_sq {
                        continue;
                    }
                }
            }

            let idx = ((y as usize * buf_w as usize + x as usize) * 4) as usize;
            buf[idx] = (premul_b + (buf[idx] as u32 * inv_a) / 255) as u8;
            buf[idx + 1] = (premul_g + (buf[idx + 1] as u32 * inv_a) / 255) as u8;
            buf[idx + 2] = (premul_r + (buf[idx + 2] as u32 * inv_a) / 255) as u8;
            buf[idx + 3] = (a as u32 + (buf[idx + 3] as u32 * inv_a) / 255) as u8;
        }
    }
}

pub fn draw_premultiplied_border(
    buf: &mut [u8],
    buf_w: u32,
    buf_h: u32,
    rect: [i32; 4],
    thickness: i32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
    radius: i32,
) {
    let [rx, ry, rw, rh] = rect;
    if rw <= 0 || rh <= 0 || a == 0 || thickness <= 0 {
        return;
    }

    let min_x = rx.max(0);
    let max_x = (rx + rw).min(buf_w as i32);
    let min_y = ry.max(0);
    let max_y = (ry + rh).min(buf_h as i32);
    if min_x >= max_x || min_y >= max_y {
        return;
    }

    let premul_r = (r as u32 * a as u32) / 255;
    let premul_g = (g as u32 * a as u32) / 255;
    let premul_b = (b as u32 * a as u32) / 255;
    let inv_a = 255 - a as u32;

    let outer_rad_sq = radius * radius;
    let inner_rad = (radius - thickness).max(0);
    let inner_rad_sq = inner_rad * inner_rad;

    for y in min_y..max_y {
        let rel_y = y - ry;
        for x in min_x..max_x {
            let rel_x = x - rx;

            let is_edge = rel_x < thickness
                || rel_x >= rw - thickness
                || rel_y < thickness
                || rel_y >= rh - thickness;

            let in_top_left = rel_x < radius && rel_y < radius;
            let in_top_right = rel_x >= rw - radius && rel_y < radius;
            let in_bot_left = rel_x < radius && rel_y >= rh - radius;
            let in_bot_right = rel_x >= rw - radius && rel_y >= rh - radius;

            let is_corner = in_top_left || in_top_right || in_bot_left || in_bot_right;

            if !is_edge && !is_corner {
                continue;
            }

            if is_corner && radius > 0 {
                let (dx, dy) = if in_top_left {
                    (radius - 1 - rel_x, radius - 1 - rel_y)
                } else if in_top_right {
                    (rel_x - (rw - radius), radius - 1 - rel_y)
                } else if in_bot_left {
                    (radius - 1 - rel_x, rel_y - (rh - radius))
                } else {
                    (rel_x - (rw - radius), rel_y - (rh - radius))
                };
                let d_sq = dx * dx + dy * dy;
                if d_sq > outer_rad_sq || d_sq < inner_rad_sq {
                    continue;
                }
            }

            let idx = ((y as usize * buf_w as usize + x as usize) * 4) as usize;
            buf[idx] = (premul_b + (buf[idx] as u32 * inv_a) / 255) as u8;
            buf[idx + 1] = (premul_g + (buf[idx + 1] as u32 * inv_a) / 255) as u8;
            buf[idx + 2] = (premul_r + (buf[idx + 2] as u32 * inv_a) / 255) as u8;
            buf[idx + 3] = (a as u32 + (buf[idx + 3] as u32 * inv_a) / 255) as u8;
        }
    }
}

pub fn blit_rgba_sprite(
    buf: &mut [u8],
    buf_w: u32,
    buf_h: u32,
    sprite: &image::RgbaImage,
    target_x: i32,
    target_y: i32,
    target_w: u32,
    target_h: u32,
) {
    if target_w == 0 || target_h == 0 {
        return;
    }
    let sw = sprite.width();
    let sh = sprite.height();
    if sw == 0 || sh == 0 {
        return;
    }

    let min_x = target_x.max(0);
    let max_x = (target_x + target_w as i32).min(buf_w as i32);
    let min_y = target_y.max(0);
    let max_y = (target_y + target_h as i32).min(buf_h as i32);
    if min_x >= max_x || min_y >= max_y {
        return;
    }

    for y in min_y..max_y {
        let dy = (y - target_y) as u32;
        let sy = (dy * sh) / target_h;
        for x in min_x..max_x {
            let dx = (x - target_x) as u32;
            let sx = (dx * sw) / target_w;
            let pixel = sprite.get_pixel(sx, sy);
            let sa = pixel[3] as u32;
            if sa == 0 {
                continue;
            }
            let sr = pixel[0] as u32;
            let sg = pixel[1] as u32;
            let sb = pixel[2] as u32;

            let premul_r = (sr * sa) / 255;
            let premul_g = (sg * sa) / 255;
            let premul_b = (sb * sa) / 255;
            let inv_a = 255 - sa;

            let idx = ((y as usize * buf_w as usize + x as usize) * 4) as usize;
            buf[idx] = (premul_b + (buf[idx] as u32 * inv_a) / 255) as u8;
            buf[idx + 1] = (premul_g + (buf[idx + 1] as u32 * inv_a) / 255) as u8;
            buf[idx + 2] = (premul_r + (buf[idx + 2] as u32 * inv_a) / 255) as u8;
            buf[idx + 3] = (sa + (buf[idx + 3] as u32 * inv_a) / 255) as u8;
        }
    }
}

// ---------------------------------------------------------
// Host Process Supervisor
// ---------------------------------------------------------

struct ChildProcessHandle {
    child: std::process::Child,
    stdin: std::io::BufWriter<std::process::ChildStdin>,
}

lazy_static::lazy_static! {
    static ref SUPERVISOR: Mutex<Option<ChildProcessHandle>> = Mutex::new(None);
    static ref LAST_CALIBRATED_RECT: Mutex<Option<(i32, i32, i32, i32)>> = Mutex::new(None);
}

fn resolve_overlay_binary_path() -> std::path::PathBuf {
    let binary_name = if cfg!(windows) { "lcu-overlay.exe" } else { "lcu-overlay" };
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name(binary_name);
        if sibling.exists() {
            return sibling;
        }
        if let Some(parent) = exe.parent().and_then(|p| p.parent()) {
            let candidate = parent.join(binary_name);
            if candidate.exists() {
                return candidate;
            }
        }
        sibling
    } else {
        std::path::PathBuf::from(binary_name)
    }
}

pub fn ensure_overlay_process_started() -> Result<(), String> {
    let mut lock = SUPERVISOR.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut handle) = *lock {
        if let Ok(None) = handle.child.try_wait() {
            return Ok(());
        }
    }

    let exe_path = resolve_overlay_binary_path();
    let mut cmd = std::process::Command::new(&exe_path);
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn {:?}: {e}", exe_path))?;
    let stdin = child.stdin.take().ok_or_else(|| "Failed to capture child stdin".to_string())?;
    let stdout = child.stdout.take();

    if let Some(stdout) = stdout {
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(l) = line {
                    if let Ok(evt) = serde_json::from_str::<OverlayIpcEvent>(&l) {
                        crate::logger::write(
                            crate::logger::Level::Info,
                            "native_overlay",
                            &format!("Overlay child event: {:?}", evt),
                            None,
                        );
                    }
                }
            }
        });
    }

    *lock = Some(ChildProcessHandle {
        child,
        stdin: std::io::BufWriter::new(stdin),
    });

    if let Ok(mut c_lock) = LAST_CALIBRATED_RECT.lock() {
        *c_lock = None;
    }

    Ok(())
}

pub fn send_overlay_command(cmd: &OverlayIpcCommand) -> Result<(), String> {
    let mut lock = SUPERVISOR.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut handle) = *lock {
        let json = serde_json::to_string(cmd).map_err(|e| e.to_string())?;
        handle.stdin.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        handle.stdin.write_all(b"\n").map_err(|e| e.to_string())?;
        handle.stdin.flush().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Overlay process not started".to_string())
    }
}

pub fn calibrate_if_changed(x: i32, y: i32, w: i32, h: i32) {
    let should_calibrate = {
        let mut cache = match LAST_CALIBRATED_RECT.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        if *cache != Some((x, y, w, h)) {
            *cache = Some((x, y, w, h));
            true
        } else {
            false
        }
    };

    if should_calibrate {
        let _ = send_overlay_command(&OverlayIpcCommand::CalibrateOnce {
            game_x: x,
            game_y: y,
            game_width: w,
            game_height: h,
        });
    }
}

pub fn update_native_overlay_state(payload: OverlayRenderPayload) -> Result<(), String> {
    send_overlay_command(&OverlayIpcCommand::Render { payload })
}

pub fn hide_native_overlay() {
    let _ = send_overlay_command(&OverlayIpcCommand::Hide);
}

pub fn shutdown_native_overlay() {
    let _ = send_overlay_command(&OverlayIpcCommand::Shutdown);
    if let Ok(mut lock) = SUPERVISOR.lock() {
        if let Some(mut handle) = lock.take() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = handle.child.kill();
        }
    }
    if let Ok(mut c_lock) = LAST_CALIBRATED_RECT.lock() {
        *c_lock = None;
    }
}

// ---------------------------------------------------------
// Native Win32 Detached Renderer
// ---------------------------------------------------------

#[cfg(windows)]
static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);

lazy_static::lazy_static! {
    static ref PENDING_COMMANDS: Mutex<Vec<OverlayIpcCommand>> = Mutex::new(Vec::new());
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum FontKind {
    Normal,
    Bold,
    Small,
    Title,
}

#[cfg(windows)]
struct ScratchTextRenderer {
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    hbm: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    pixels: *mut u8,
    width: i32,
    height: i32,
    font_normal: windows_sys::Win32::Graphics::Gdi::HFONT,
    font_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    font_small: windows_sys::Win32::Graphics::Gdi::HFONT,
    font_title: windows_sys::Win32::Graphics::Gdi::HFONT,
}

#[cfg(windows)]
impl ScratchTextRenderer {
    fn new(screen_dc: windows_sys::Win32::Graphics::Gdi::HDC) -> Option<Self> {
        use windows_sys::Win32::Graphics::Gdi::*;
        unsafe {
            let hdc = CreateCompatibleDC(screen_dc);
            if hdc.is_null() {
                return None;
            }
            let width = 640;
            let height = 64;

            let mut bi = std::mem::zeroed::<BITMAPINFO>();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = width;
            bi.bmiHeader.biHeight = -height;
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB;

            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let hbm = CreateDIBSection(
                hdc,
                &bi,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if hbm.is_null() || bits.is_null() {
                DeleteDC(hdc);
                return None;
            }
            SelectObject(hdc, hbm);
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, 0x00FFFFFF);

            let font_name: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
            let create_f = |size: i32, weight: i32| {
                CreateFontW(
                    size,
                    0,
                    0,
                    0,
                    weight,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    CLEARTYPE_QUALITY as u32,
                    DEFAULT_PITCH as u32 | FF_DONTCARE as u32,
                    font_name.as_ptr(),
                )
            };

            let font_normal = create_f(-14, FW_NORMAL as i32);
            let font_bold = create_f(-14, FW_BOLD as i32);
            let font_small = create_f(-11, FW_NORMAL as i32);
            let font_title = create_f(-16, FW_BOLD as i32);

            Some(Self {
                hdc,
                hbm,
                pixels: bits as *mut u8,
                width,
                height,
                font_normal,
                font_bold,
                font_small,
                font_title,
            })
        }
    }

    fn draw_text_line(
        &mut self,
        buf: &mut [u8],
        buf_w: u32,
        buf_h: u32,
        text: &str,
        x: i32,
        y: i32,
        font_kind: FontKind,
        r: u8,
        g: u8,
        b: u8,
        max_w: Option<i32>,
    ) {
        if text.is_empty() {
            return;
        }
        use windows_sys::Win32::Graphics::Gdi::*;
        unsafe {
            std::ptr::write_bytes(self.pixels, 0, (self.width * self.height * 4) as usize);

            let font = match font_kind {
                FontKind::Normal => self.font_normal,
                FontKind::Bold => self.font_bold,
                FontKind::Small => self.font_small,
                FontKind::Title => self.font_title,
            };
            SelectObject(self.hdc, font);

            let wide: Vec<u16> = text.encode_utf16().collect();
            let limit_w = max_w.unwrap_or(self.width).min(self.width);
            let mut rc = windows_sys::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: limit_w,
                bottom: self.height,
            };

            DrawTextW(
                self.hdc,
                wide.as_ptr(),
                wide.len() as i32,
                &mut rc,
                DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX,
            );

            let drawn_w = (rc.right - rc.left).min(self.width);
            let drawn_h = (rc.bottom - rc.top).min(self.height);

            for sy in 0..drawn_h {
                let dst_y = y + sy;
                if dst_y < 0 || dst_y >= buf_h as i32 {
                    continue;
                }
                for sx in 0..drawn_w {
                    let dst_x = x + sx;
                    if dst_x < 0 || dst_x >= buf_w as i32 {
                        continue;
                    }
                    let s_idx = ((sy * self.width + sx) * 4) as usize;
                    let alpha = ((self.pixels.add(s_idx).read() as u32 * 299
                        + self.pixels.add(s_idx + 1).read() as u32 * 587
                        + self.pixels.add(s_idx + 2).read() as u32 * 114)
                        / 1000) as u8;
                    if alpha == 0 {
                        continue;
                    }

                    let premul_r = (r as u32 * alpha as u32) / 255;
                    let premul_g = (g as u32 * alpha as u32) / 255;
                    let premul_b = (b as u32 * alpha as u32) / 255;
                    let inv_a = 255 - alpha as u32;

                    let d_idx = ((dst_y as usize * buf_w as usize + dst_x as usize) * 4) as usize;
                    buf[d_idx] = (premul_b + (buf[d_idx] as u32 * inv_a) / 255) as u8;
                    buf[d_idx + 1] = (premul_g + (buf[d_idx + 1] as u32 * inv_a) / 255) as u8;
                    buf[d_idx + 2] = (premul_r + (buf[d_idx + 2] as u32 * inv_a) / 255) as u8;
                    buf[d_idx + 3] = (alpha as u32 + (buf[d_idx + 3] as u32 * inv_a) / 255) as u8;
                }
            }
        }
    }
}

#[cfg(windows)]
impl Drop for ScratchTextRenderer {
    fn drop(&mut self) {
        use windows_sys::Win32::Graphics::Gdi::*;
        unsafe {
            DeleteObject(self.font_normal);
            DeleteObject(self.font_bold);
            DeleteObject(self.font_small);
            DeleteObject(self.font_title);
            DeleteObject(self.hbm);
            DeleteDC(self.hdc);
        }
    }
}

#[cfg(windows)]
struct OverlayWinContext {
    hwnd: windows_sys::Win32::Foundation::HWND,
    layout: Option<OverlayAnchorLayout>,
    dib_dc: windows_sys::Win32::Graphics::Gdi::HDC,
    dib_hbm: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    dib_pixels: *mut u8,
    dib_width: i32,
    dib_height: i32,
    text_renderer: Option<ScratchTextRenderer>,
    sprite_cache: HashMap<String, image::RgbaImage>,
    current_payload: Option<OverlayRenderPayload>,
}

#[cfg(windows)]
impl OverlayWinContext {
    fn new(hwnd: windows_sys::Win32::Foundation::HWND) -> Self {
        use windows_sys::Win32::Graphics::Gdi::*;
        let screen_dc = unsafe { GetDC(std::ptr::null_mut()) };
        let text_renderer = ScratchTextRenderer::new(screen_dc);
        unsafe { ReleaseDC(std::ptr::null_mut(), screen_dc) };

        Self {
            hwnd,
            layout: None,
            dib_dc: std::ptr::null_mut(),
            dib_hbm: std::ptr::null_mut(),
            dib_pixels: std::ptr::null_mut(),
            dib_width: 0,
            dib_height: 0,
            text_renderer,
            sprite_cache: HashMap::new(),
            current_payload: None,
        }
    }

    fn ensure_dib_size(&mut self, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        if self.dib_width == width && self.dib_height == height && !self.dib_pixels.is_null() {
            return;
        }

        use windows_sys::Win32::Graphics::Gdi::*;
        unsafe {
            if !self.dib_hbm.is_null() {
                DeleteObject(self.dib_hbm);
                self.dib_hbm = std::ptr::null_mut();
            }
            if !self.dib_dc.is_null() {
                DeleteDC(self.dib_dc);
                self.dib_dc = std::ptr::null_mut();
            }

            let screen_dc = GetDC(std::ptr::null_mut());
            self.dib_dc = CreateCompatibleDC(screen_dc);

            let mut bi = std::mem::zeroed::<BITMAPINFO>();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = width;
            bi.bmiHeader.biHeight = -height;
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB;

            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            self.dib_hbm = CreateDIBSection(
                self.dib_dc,
                &bi,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            SelectObject(self.dib_dc, self.dib_hbm);
            ReleaseDC(std::ptr::null_mut(), screen_dc);

            self.dib_pixels = bits as *mut u8;
            self.dib_width = width;
            self.dib_height = height;
        }
    }

    fn render(&mut self) {
        let payload = match self.current_payload.clone() {
            Some(p) if p.visible => p,
            _ => {
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                        self.hwnd,
                        windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE,
                    );
                }
                return;
            }
        };

        let layout = match self.layout.as_ref() {
            Some(l) => l.clone(),
            None => return,
        };

        let w = layout.game_width;
        let h = layout.game_height;
        if w <= 0 || h <= 0 {
            return;
        }

        self.ensure_dib_size(w, h);
        if self.dib_pixels.is_null() {
            return;
        }

        let total_bytes = (w * h * 4) as usize;
        unsafe {
            std::ptr::write_bytes(self.dib_pixels, 0, total_bytes);
        }
        let buf = unsafe { std::slice::from_raw_parts_mut(self.dib_pixels, total_bytes) };

        // 1. Top-Right HUD Pill
        let [bx, by, bw, bh] = layout.hud_banner_rect;
        fill_premultiplied_rect(buf, w as u32, h as u32, [bx, by, bw, bh], 9, 13, 22, 230, 10);
        let (border_r, border_g, border_b) = if payload.scanning {
            (245, 158, 11) // Amber
        } else {
            (6, 182, 212) // Cyan
        };
        draw_premultiplied_border(
            buf,
            w as u32,
            h as u32,
            [bx, by, bw, bh],
            1,
            border_r,
            border_g,
            border_b,
            220,
            10,
        );

        if let Some(ref mut tr) = self.text_renderer {
            if payload.scanning {
                tr.draw_text_line(
                    buf,
                    w as u32,
                    h as u32,
                    "SCANNING AUGMENTS...",
                    bx + 14,
                    by + 12,
                    FontKind::Bold,
                    245,
                    158,
                    11,
                    Some(bw - 28),
                );
            } else {
                let champ = if payload.champion_name.is_empty() {
                    "ARAM MAYHEM"
                } else {
                    &payload.champion_name
                };
                let status_text = format!("LCU OVERLAY • {} • [F8] SCAN  [F7] HIDE", champ.to_uppercase());
                tr.draw_text_line(
                    buf,
                    w as u32,
                    h as u32,
                    &status_text,
                    bx + 14,
                    by + 12,
                    FontKind::Bold,
                    220,
                    225,
                    235,
                    Some(bw - 28),
                );
            }
        }

        // 2. Blit PNG Sprites
        for sprite in &payload.sprites {
            if !self.sprite_cache.contains_key(&sprite.id) {
                if let Some(ref bytes) = sprite.png_base64_or_bytes {
                    if let Ok(img) = image::load_from_memory(bytes) {
                        self.sprite_cache.insert(sprite.id.clone(), img.to_rgba8());
                    }
                } else if let Some(ref path) = sprite.png_path {
                    if let Ok(img) = image::open(path) {
                        self.sprite_cache.insert(sprite.id.clone(), img.to_rgba8());
                    }
                }
            }

            if let Some(img) = self.sprite_cache.get(&sprite.id) {
                blit_rgba_sprite(
                    buf,
                    w as u32,
                    h as u32,
                    img,
                    sprite.x,
                    sprite.y,
                    sprite.width,
                    sprite.height,
                );
            }
        }

        // 3. Draw Augment Cards
        if !payload.cards.is_empty() {
            let mut sorted_cards = payload.cards.clone();
            sorted_cards.sort_by(|a, b| b.wpa.partial_cmp(&a.wpa).unwrap_or(std::cmp::Ordering::Equal));
            let best_id = sorted_cards.first().map(|c| c.id.clone()).unwrap_or_default();

            for card in &payload.cards {
                let slot = card.slot.min(2);
                let rect = layout.card_rects[slot];
                let [cx, cy, cw, _ch] = rect;
                let is_best = card.id == best_id;

                let rarity_lower = card.rarity.as_deref().unwrap_or("").to_lowercase();
                let (cr_r, cr_g, cr_b) = if is_best {
                    (16, 185, 129) // Emerald
                } else if rarity_lower.contains("prismatic") {
                    (168, 85, 247) // Prismatic
                } else if rarity_lower.contains("gold") {
                    (245, 158, 11) // Gold
                } else {
                    (148, 163, 184) // Silver
                };

                fill_premultiplied_rect(buf, w as u32, h as u32, rect, 10, 14, 23, 238, 10);
                draw_premultiplied_border(buf, w as u32, h as u32, rect, 2, cr_r, cr_g, cr_b, 240, 10);

                if let Some(ref mut tr) = self.text_renderer {
                    let p = 12;
                    let mut cur_y = cy + p;

                    // Header badge
                    if is_best {
                        tr.draw_text_line(
                            buf,
                            w as u32,
                            h as u32,
                            "★ #1 BEST PICK",
                            cx + p,
                            cur_y,
                            FontKind::Bold,
                            16,
                            185,
                            129,
                            Some(cw - p * 2),
                        );
                    } else {
                        let rank = sorted_cards.iter().position(|c| c.id == card.id).unwrap_or(slot) + 1;
                        let badge = format!("OPTION #{}", rank);
                        tr.draw_text_line(
                            buf,
                            w as u32,
                            h as u32,
                            &badge,
                            cx + p,
                            cur_y,
                            FontKind::Bold,
                            156,
                            163,
                            175,
                            Some(cw - p * 2),
                        );
                    }

                    let rarity_badge = card.rarity.as_deref().unwrap_or("SILVER").to_uppercase();
                    tr.draw_text_line(
                        buf,
                        w as u32,
                        h as u32,
                        &rarity_badge,
                        cx + cw - p - 65,
                        cur_y,
                        FontKind::Small,
                        cr_r,
                        cr_g,
                        cr_b,
                        Some(65),
                    );

                    cur_y += 20;
                    tr.draw_text_line(
                        buf,
                        w as u32,
                        h as u32,
                        &card.name,
                        cx + p,
                        cur_y,
                        FontKind::Title,
                        241,
                        245,
                        249,
                        Some(cw - p * 2),
                    );

                    cur_y += 22;
                    let wr_text = format!("WR {:.1}%", card.win_rate * 100.0);
                    tr.draw_text_line(
                        buf,
                        w as u32,
                        h as u32,
                        &wr_text,
                        cx + p,
                        cur_y,
                        FontKind::Bold,
                        220,
                        225,
                        235,
                        Some(80),
                    );

                    let wpa_sign = if card.wpa >= 0.0 { "+" } else { "" };
                    let wpa_text = format!("{}{:.1}% WPA", wpa_sign, card.wpa * 100.0);
                    let (wpa_r, wpa_g, wpa_b) = if card.wpa >= 0.0 { (16, 185, 129) } else { (248, 113, 113) };
                    tr.draw_text_line(
                        buf,
                        w as u32,
                        h as u32,
                        &wpa_text,
                        cx + p + 82,
                        cur_y,
                        FontKind::Bold,
                        wpa_r,
                        wpa_g,
                        wpa_b,
                        Some(95),
                    );

                    let games_text = format!("{}g", card.games);
                    tr.draw_text_line(
                        buf,
                        w as u32,
                        h as u32,
                        &games_text,
                        cx + cw - p - 45,
                        cur_y,
                        FontKind::Small,
                        156,
                        163,
                        175,
                        Some(45),
                    );

                    cur_y += 18;
                    fill_premultiplied_rect(buf, w as u32, h as u32, [cx + p, cur_y, cw - p * 2, 1], 31, 41, 55, 200, 0);

                    cur_y += 8;
                    tr.draw_text_line(
                        buf,
                        w as u32,
                        h as u32,
                        "TOP COMBOS & SYNERGIES",
                        cx + p,
                        cur_y,
                        FontKind::Small,
                        156,
                        163,
                        175,
                        Some(cw - p * 2),
                    );

                    cur_y += 16;
                    if !card.combos.is_empty() {
                        for combo in card.combos.iter().take(2) {
                            let wr_str = format!("{:.1}% WR", combo.win_rate * 100.0);
                            tr.draw_text_line(
                                buf,
                                w as u32,
                                h as u32,
                                &combo.label,
                                cx + p,
                                cur_y,
                                FontKind::Small,
                                209,
                                213,
                                219,
                                Some(cw - p * 2 - 65),
                            );
                            tr.draw_text_line(
                                buf,
                                w as u32,
                                h as u32,
                                &wr_str,
                                cx + cw - p - 55,
                                cur_y,
                                FontKind::Small,
                                16,
                                185,
                                129,
                                Some(55),
                            );
                            cur_y += 14;
                        }
                    } else if !card.synergy_items.is_empty() {
                        let items_str = card
                            .synergy_items
                            .iter()
                            .map(|i| i.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" + ");
                        tr.draw_text_line(
                            buf,
                            w as u32,
                            h as u32,
                            &items_str,
                            cx + p,
                            cur_y,
                            FontKind::Small,
                            209,
                            213,
                            219,
                            Some(cw - p * 2),
                        );
                    }
                }
            }
        }

        // 4. UpdateLayeredWindow
        unsafe {
            use windows_sys::Win32::Graphics::Gdi::*;
            use windows_sys::Win32::UI::WindowsAndMessaging::*;

            let screen_dc = GetDC(std::ptr::null_mut());
            let mut pt_dst = windows_sys::Win32::Foundation::POINT {
                x: layout.game_x,
                y: layout.game_y,
            };
            let mut size = windows_sys::Win32::Foundation::SIZE {
                cx: w,
                cy: h,
            };
            let mut pt_src = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            UpdateLayeredWindow(
                self.hwnd,
                screen_dc,
                &mut pt_dst,
                &mut size,
                self.dib_dc,
                &mut pt_src,
                0,
                &blend,
                ULW_ALPHA,
            );
            ReleaseDC(std::ptr::null_mut(), screen_dc);

            ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }
}

#[cfg(windows)]
impl Drop for OverlayWinContext {
    fn drop(&mut self) {
        use windows_sys::Win32::Graphics::Gdi::*;
        unsafe {
            if !self.dib_hbm.is_null() {
                DeleteObject(self.dib_hbm);
            }
            if !self.dib_dc.is_null() {
                DeleteDC(self.dib_dc);
            }
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn overlay_wnd_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    match msg {
        WM_CREATE => {
            let cs = lparam as *const CREATESTRUCTW;
            if !cs.is_null() {
                let ctx_ptr = (*cs).lpCreateParams as *mut OverlayWinContext;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, ctx_ptr as isize);
            }
            0
        }
        _ if msg == WM_APP + 1 => {
            let ctx_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayWinContext;
            if !ctx_ptr.is_null() {
                let ctx = &mut *ctx_ptr;
                let cmds = {
                    let mut q = PENDING_COMMANDS.lock().unwrap();
                    std::mem::take(&mut *q)
                };

                for cmd in cmds {
                    match cmd {
                        OverlayIpcCommand::CalibrateOnce {
                            game_x,
                            game_y,
                            game_width,
                            game_height,
                        } => {
                            let layout = calibrate_layout_from_single_screenshot(
                                game_x,
                                game_y,
                                game_width,
                                game_height,
                            );
                            ctx.layout = Some(layout.clone());
                            let evt = OverlayIpcEvent::Calibrated { layout };
                            if let Ok(json) = serde_json::to_string(&evt) {
                                println!("{json}");
                                let _ = std::io::stdout().flush();
                            }
                            SetWindowPos(
                                hwnd,
                                HWND_TOPMOST,
                                game_x,
                                game_y,
                                game_width,
                                game_height,
                                SWP_NOACTIVATE,
                            );
                            if let Some(ref p) = ctx.current_payload {
                                if p.visible {
                                    ctx.render();
                                }
                            }
                        }
                        OverlayIpcCommand::Render { payload } => {
                            let visible = payload.visible;
                            ctx.current_payload = Some(payload);
                            if visible {
                                ctx.render();
                            } else {
                                ShowWindow(hwnd, SW_HIDE);
                            }
                        }
                        OverlayIpcCommand::Hide => {
                            if let Some(ref mut p) = ctx.current_payload {
                                p.visible = false;
                            }
                            ShowWindow(hwnd, SW_HIDE);
                        }
                        OverlayIpcCommand::Shutdown => {
                            DestroyWindow(hwnd);
                            PostQuitMessage(0);
                        }
                    }
                }
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn run_detached_overlay_process() {
    let pid = std::process::id();
    let ready_evt = OverlayIpcEvent::Ready { pid };
    if let Ok(json) = serde_json::to_string(&ready_evt) {
        println!("{json}");
        let _ = std::io::stdout().flush();
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows_sys::Win32::UI::WindowsAndMessaging::*;

        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class_name: Vec<u16> = "LcuNativeOverlayClass\0".encode_utf16().collect();

            let wnd_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(overlay_wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };

            RegisterClassW(&wnd_class);

            let mut context = Box::new(OverlayWinContext::new(std::ptr::null_mut()));
            let ctx_raw: *mut OverlayWinContext = &mut *context;

            let ex_style = WS_EX_LAYERED
                | WS_EX_TRANSPARENT
                | WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE;

            let hwnd = CreateWindowExW(
                ex_style,
                class_name.as_ptr(),
                "LCU Companion Native Overlay\0".encode_utf16().collect::<Vec<u16>>().as_ptr(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                ctx_raw as *mut std::ffi::c_void,
            );

            if hwnd.is_null() {
                eprintln!("Failed to create overlay HWND");
                return;
            }

            context.hwnd = hwnd;
            OVERLAY_HWND.store(hwnd as isize, Ordering::Relaxed);

            // Stdin listener thread
            std::thread::spawn(move || {
                use std::io::BufRead;
                let stdin = std::io::stdin();
                for line in stdin.lock().lines() {
                    let line = match line {
                        Ok(l) => l,
                        Err(_) => break,
                    };
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(cmd) = serde_json::from_str::<OverlayIpcCommand>(&line) {
                        let should_shutdown = matches!(cmd, OverlayIpcCommand::Shutdown);
                        {
                            let mut q = PENDING_COMMANDS.lock().unwrap();
                            q.push(cmd);
                        }
                        let h = OVERLAY_HWND.load(Ordering::Relaxed) as windows_sys::Win32::Foundation::HWND;
                        if !h.is_null() {
                            PostMessageW(h, WM_APP + 1, 0, 0);
                        }
                        if should_shutdown {
                            break;
                        }
                    }
                }
            });

            // Windows message loop
            let mut msg = std::mem::zeroed::<MSG>();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    #[cfg(not(windows))]
    {
        use std::io::BufRead;
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            if let Ok(l) = line {
                if let Ok(OverlayIpcCommand::Shutdown) = serde_json::from_str::<OverlayIpcCommand>(&l) {
                    break;
                }
            }
        }
    }
}

// ---------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_command_event_roundtrip() {
        // Test Commands
        let calibrate_cmd = OverlayIpcCommand::CalibrateOnce {
            game_x: 100,
            game_y: 200,
            game_width: 1920,
            game_height: 1080,
        };
        let cal_json = serde_json::to_string(&calibrate_cmd).expect("serialize CalibrateOnce");
        let parsed_cal: OverlayIpcCommand = serde_json::from_str(&cal_json).expect("deserialize CalibrateOnce");
        if let OverlayIpcCommand::CalibrateOnce { game_width, game_height, .. } = parsed_cal {
            assert_eq!(game_width, 1920);
            assert_eq!(game_height, 1080);
        } else {
            panic!("Expected CalibrateOnce command");
        }

        let render_cmd = OverlayIpcCommand::Render {
            payload: OverlayRenderPayload {
                visible: true,
                scanning: false,
                champion_name: "Jinx".to_string(),
                strategy_label: "Lethality".to_string(),
                cards: vec![OverlayAugmentCard {
                    slot: 0,
                    id: "1".to_string(),
                    name: "Phenomenal Evil".to_string(),
                    rarity: Some("Gold".to_string()),
                    wpa: 0.045,
                    win_rate: 0.582,
                    games: 120,
                    synergy_items: Vec::new(),
                    combos: Vec::new(),
                }],
                sprites: vec![OverlayPngSprite {
                    id: "icon1".to_string(),
                    png_path: Some("/test/icon.png".to_string()),
                    png_base64_or_bytes: None,
                    x: 10,
                    y: 20,
                    width: 32,
                    height: 32,
                }],
            },
        };
        let render_json = serde_json::to_string(&render_cmd).expect("serialize Render");
        let parsed_render: OverlayIpcCommand = serde_json::from_str(&render_json).expect("deserialize Render");
        if let OverlayIpcCommand::Render { payload } = parsed_render {
            assert_eq!(payload.champion_name, "Jinx");
            assert_eq!(payload.cards.len(), 1);
            assert_eq!(payload.cards[0].name, "Phenomenal Evil");
            assert_eq!(payload.sprites.len(), 1);
            assert_eq!(payload.sprites[0].id, "icon1");
        } else {
            panic!("Expected Render command");
        }

        let hide_cmd = OverlayIpcCommand::Hide;
        let hide_json = serde_json::to_string(&hide_cmd).unwrap();
        assert!(matches!(serde_json::from_str::<OverlayIpcCommand>(&hide_json).unwrap(), OverlayIpcCommand::Hide));

        let shutdown_cmd = OverlayIpcCommand::Shutdown;
        let shut_json = serde_json::to_string(&shutdown_cmd).unwrap();
        assert!(matches!(serde_json::from_str::<OverlayIpcCommand>(&shut_json).unwrap(), OverlayIpcCommand::Shutdown));

        // Test Events
        let ready_evt = OverlayIpcEvent::Ready { pid: 9999 };
        let ready_json = serde_json::to_string(&ready_evt).unwrap();
        assert_eq!(ready_evt, serde_json::from_str(&ready_json).unwrap());

        let layout = compute_default_anchor_layout(0, 0, 1920, 1080);
        let cal_evt = OverlayIpcEvent::Calibrated { layout: layout.clone() };
        let cal_evt_json = serde_json::to_string(&cal_evt).unwrap();
        assert_eq!(cal_evt, serde_json::from_str(&cal_evt_json).unwrap());

        let err_evt = OverlayIpcEvent::Error { message: "failed capture".to_string() };
        let err_json = serde_json::to_string(&err_evt).unwrap();
        assert_eq!(err_evt, serde_json::from_str(&err_json).unwrap());
    }

    #[test]
    fn test_anchor_layout_calculations_1080p_and_1440p() {
        // 1920x1080
        let l1080 = compute_default_anchor_layout(0, 0, 1920, 1080);
        assert_eq!(l1080.game_width, 1920);
        assert_eq!(l1080.game_height, 1080);
        assert_eq!(l1080.hud_banner_rect, [1920 - 340 - 24, 18, 340, 42]);
        assert_eq!(l1080.title_strip_rect, [(1920.0 * 0.21) as i32, (1080.0 * 0.385) as i32, (1920.0 * 0.58) as i32, (1080.0 * 0.065) as i32]);
        assert_eq!(l1080.card_rects[0], [(1920.0 * 0.21) as i32, (1080.0 * 0.08) as i32, (1920.0 * 0.17) as i32, (1080.0 * 0.24) as i32]);
        assert_eq!(l1080.card_rects[1], [(1920.0 * 0.415) as i32, (1080.0 * 0.08) as i32, (1920.0 * 0.17) as i32, (1080.0 * 0.24) as i32]);
        assert_eq!(l1080.card_rects[2], [(1920.0 * 0.62) as i32, (1080.0 * 0.08) as i32, (1920.0 * 0.17) as i32, (1080.0 * 0.24) as i32]);

        // 2560x1440
        let l1440 = compute_default_anchor_layout(100, 50, 2560, 1440);
        assert_eq!(l1440.game_x, 100);
        assert_eq!(l1440.game_y, 50);
        assert_eq!(l1440.game_width, 2560);
        assert_eq!(l1440.game_height, 1440);
        assert_eq!(l1440.hud_banner_rect, [2560 - 340 - 24, 18, 340, 42]);
        assert_eq!(l1440.card_rects[0][2], (2560.0 * 0.17) as i32);
        assert_eq!(l1440.card_rects[0][3], (1440.0 * 0.24) as i32);
    }

    #[test]
    fn test_premultiplied_bgra_rect_and_sprite_blitting() {
        let width = 100u32;
        let height = 100u32;
        let mut buf = vec![0u8; (width * height * 4) as usize];

        // Fill rect [10, 10, 20, 20] with rgba(255, 0, 0, 128)
        fill_premultiplied_rect(&mut buf, width, height, [10, 10, 20, 20], 255, 0, 0, 128, 0);

        let pixel_idx = ((15 * width + 15) * 4) as usize;
        let b = buf[pixel_idx];
        let g = buf[pixel_idx + 1];
        let r = buf[pixel_idx + 2];
        let a = buf[pixel_idx + 3];

        assert_eq!(b, 0);
        assert_eq!(g, 0);
        assert_eq!(r, ((255u32 * 128) / 255) as u8); // 128 premultiplied red
        assert_eq!(a, 128);

        // Blit a 4x4 blue sprite rgba(0, 0, 255, 255) at [14, 14, 4, 4]
        let mut sprite = image::RgbaImage::new(4, 4);
        for pixel in sprite.pixels_mut() {
            *pixel = image::Rgba([0, 0, 255, 255]);
        }

        blit_rgba_sprite(&mut buf, width, height, &sprite, 14, 14, 4, 4);

        let blit_idx = ((15 * width + 15) * 4) as usize;
        let bb = buf[blit_idx];
        let bg = buf[blit_idx + 1];
        let br = buf[blit_idx + 2];
        let ba = buf[blit_idx + 3];

        // Blue sprite with alpha 255 completely covers underlying red
        assert_eq!(bb, 255);
        assert_eq!(bg, 0);
        assert_eq!(br, 0);
        assert_eq!(ba, 255);
    }
}
