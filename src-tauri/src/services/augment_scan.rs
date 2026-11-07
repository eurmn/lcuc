//! # In-game augment card identification
//!
//! The overlay has to answer one question several times per match: *which three
//! augments are on the selection screen right now?*
//!
//! The previous engine OCR'd the whole card area (titles, badges and description
//! bodies), then scored every candidate augment against the recognised text with
//! Levenshtein distance, over a 1000-row database projection. That is slow
//! (hundreds of ms of OCR plus hundreds of ms of string scoring, on the game's
//! polling thread) and unreliable (description bodies and animated card art
//! produce noisy lines).
//!
//! This module uses two cheap, independent signals instead:
//!
//! 1. **Card art.** `kiwi.bin.json` — the mode's own augment table — gives every
//!    ARAM Mayhem augment its card art (`AugmentLargeIconPath`). The art drawn on
//!    the card is pixel-identical to that asset, so a 32x32 masked fingerprint
//!    identifies it deterministically without any OCR.
//! 2. **Title text.** Riot reuses some icons for several augments, so when the
//!    art alone is ambiguous we OCR just the title strip (a ~13% tall sliver of
//!    the card) and pick among that small candidate set.
//!
//! Card rectangles are located from the card borders, so the scan does not
//! depend on hard-coded pixel offsets and works at any 16:9 resolution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::services::history::{get_aram_augment_pool, AugmentScanEntry};

/// Edge length of the icon fingerprint. 32x32 is enough to separate every piece
/// of card art while keeping a full-pool scan in the sub-millisecond range.
pub const THUMB: usize = 32;

// Relative card geometry. Measured against the reference screenshots
// (1280x720 and 1920x1080), where both hold to within a pixel.
const ICON_SIZE_FRAC: f32 = 0.50; // of card width

/// Below this icon similarity there is no augment card at that slot.
const MIN_ICON_SCORE: f32 = 0.60;
/// Icon scores this close to the best are a tie and are resolved by title text.
const ICON_TIE_EPSILON: f32 = 0.035;
/// A confident art match does not need the title read at all.
const CONFIDENT_ICON_SCORE: f32 = 0.78;
/// Minimum title similarity to accept an OCR-based tie-break.
const MIN_TITLE_SCORE: f64 = 0.50;
/// Similarity at which a title matching no candidate may overrule a *weak* art
/// match, and only when that art is unique to one augment.
const STRONG_OVERRIDE_TITLE_SCORE: f64 = 0.85;

// ---------------------------------------------------------------
// Frame
// ---------------------------------------------------------------

/// A captured region of the game window, BGRA8, top-down.
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// BGRA, 4 bytes per pixel, row-major, top-down.
    pub bgra: Vec<u8>,
}

impl Frame {
    /// Wraps a raw `GetDIBits` buffer, forcing alpha to opaque. GDI leaves the
    /// fourth byte at 0, which makes WinRT OCR see a fully transparent image.
    pub fn from_bgra_capture(mut bgra: Vec<u8>, width: u32, height: u32) -> Self {
        for px in bgra.chunks_exact_mut(4) {
            px[3] = 255;
        }
        Self {
            width,
            height,
            bgra,
        }
    }

    pub fn from_rgba(rgba: &[u8], width: u32, height: u32) -> Self {
        let mut bgra = Vec::with_capacity(rgba.len());
        for px in rgba.chunks_exact(4) {
            bgra.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
        Self {
            width,
            height,
            bgra,
        }
    }

    #[inline]
    fn luma(&self, x: u32, y: u32) -> f32 {
        let i = ((y * self.width + x) * 4) as usize;
        let b = self.bgra[i] as f32;
        let g = self.bgra[i + 1] as f32;
        let r = self.bgra[i + 2] as f32;
        0.299 * r + 0.587 * g + 0.114 * b
    }

    /// Cheap uniform brightness sample, used to spot an all-black capture.
    fn mean_luma(&self) -> f32 {
        let step_x = (self.width / 64).max(1);
        let step_y = (self.height / 64).max(1);
        let mut sum = 0.0;
        let mut n = 0.0;
        let mut y = 0;
        while y < self.height {
            let mut x = 0;
            while x < self.width {
                sum += self.luma(x, y);
                n += 1.0;
                x += step_x;
            }
            y += step_y;
        }
        if n > 0.0 {
            sum / n
        } else {
            0.0
        }
    }

    fn crop_rgba(&self, x: u32, y: u32, w: u32, h: u32) -> image::RgbaImage {
        let w = w.min(self.width.saturating_sub(x)).max(1);
        let h = h.min(self.height.saturating_sub(y)).max(1);
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for row in y..y + h {
            for col in x..x + w {
                let i = ((row * self.width + col) * 4) as usize;
                out.extend_from_slice(&[self.bgra[i + 2], self.bgra[i + 1], self.bgra[i], 255]);
            }
        }
        image::RgbaImage::from_raw(w, h, out).unwrap_or_else(|| image::RgbaImage::new(1, 1))
    }
}

// ---------------------------------------------------------------
// Card geometry
// ---------------------------------------------------------------

/// One augment card as located on screen, in frame coordinates.
///
/// The card is found from the bright horizontal bar along its top edge; the art
/// and title positions are then derived from the card width, which is a fixed
/// fraction of the row's bright run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardBox {
    pub slot: usize,
    pub centre_x: u32,
    /// Estimated full card width.
    pub width: u32,
    pub icon_x: u32,
    pub icon_y: u32,
    pub icon_size: u32,
    pub title_x: u32,
    pub title_y: u32,
    pub title_w: u32,
    pub title_h: u32,
}

impl CardBox {
    pub fn icon_rect(&self) -> (u32, u32, u32) {
        (self.icon_x, self.icon_y, self.icon_size)
    }

    pub fn title_rect(&self) -> (u32, u32, u32, u32) {
        (self.title_x, self.title_y, self.title_w, self.title_h)
    }

    /// The card's body text, directly under the title. Cards whose art is a
    /// shared placeholder icon are told apart by reading this.
    pub fn description_rect(&self) -> (u32, u32, u32, u32) {
        let (_, ty, _, th) = self.title_rect();
        // Body text spans the card's inner width, not the title's: the title is
        // one short centred line, the description runs nearly edge to edge. Using
        // the title width clipped the first and last words ("Gain" -> "ain",
        // "Conqueror" -> "Conqueror"), which is enough to defeat the OCR.
        let pad = ((self.width as f32 * 0.04).round() as u32).max(2);
        let x = self.centre_x.saturating_sub(self.width / 2) + pad;
        let y = ty + th + th / 3;
        let h = ((th as f32) * 2.6).round() as u32;
        (
            x,
            y,
            self.width.saturating_sub(pad * 2).max(8),
            h.max(12),
        )
    }
}

/// Bright horizontal runs in one row, wide enough to be a card edge.
fn row_runs(frame: &Frame, y: u32, min_width: u32, min_gap: u32) -> Vec<(u32, u32)> {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    let mut start: Option<u32> = None;
    for x in 0..frame.width {
        let bright = frame.luma(x, y) >= 150.0;
        if bright && start.is_none() {
            start = Some(x);
        } else if !bright {
            if let Some(s) = start.take() {
                if x - 1 >= s {
                    runs.push((s, x - 1));
                }
            }
        }
    }
    if let Some(s) = start {
        runs.push((s, frame.width.saturating_sub(1)));
    }

    // Drop specks, then bridge the small gaps inside a card's own border.
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for run in runs {
        match merged.last_mut() {
            Some(last) if run.0.saturating_sub(last.1) <= min_gap => last.1 = run.1,
            _ => merged.push(run),
        }
    }
    merged
        .into_iter()
        .filter(|(a, b)| b - a + 1 >= min_width)
        .collect()
}

/// Card width as a fraction of the centre-to-centre pitch between cards. The
/// pitch is measured from the three detected centres, so per-card differences in
/// how bright the rounded top corner is cannot skew it.
const CARD_WIDTH_FROM_PITCH: f32 = 0.866;
/// Card art is half the card width, its top edge sits 0.12 card-widths below the
/// top border bar.
const ICON_TOP_FROM_RUN: f32 = 0.12;
/// Title line: below the art, about 40% of the art height tall.
const TITLE_TOP_FROM_ICON: f32 = 1.13;
const TITLE_HEIGHT_FROM_ICON: f32 = 0.40;

/// Locates the three cards from the bright bar along their top edge.
///
/// Returns an empty vector when the augment selection screen is not showing.
pub fn detect_cards(frame: &Frame) -> Vec<CardBox> {
    if frame.width < 200 || frame.height < 120 {
        return Vec::new();
    }
    let w = frame.width;
    let h = frame.height;
    let min_width = (w as f32 * 0.10) as u32;
    let min_gap = (w as f32 * 0.01) as u32;

    let mut border_row: Option<(u32, Vec<(u32, u32)>)> = None;
    let y0 = (h as f32 * 0.10) as u32;
    let y1 = (h as f32 * 0.42) as u32;
    for y in y0..y1 {
        let runs = row_runs(frame, y, min_width, min_gap);
        if runs.len() != 3 {
            continue;
        }
        let widths: Vec<u32> = runs.iter().map(|(a, b)| b - a + 1).collect();
        let min_w = *widths.iter().min().unwrap_or(&0);
        let max_w = *widths.iter().max().unwrap_or(&0);
        if min_w == 0 || max_w as f32 / min_w as f32 > 1.2 {
            continue;
        }
        let gap_a = runs[1].0.saturating_sub(runs[0].1);
        let gap_b = runs[2].0.saturating_sub(runs[1].1);
        let (gap_min, gap_max) = (gap_a.min(gap_b), gap_a.max(gap_b));
        if gap_min == 0 || gap_max as f32 / gap_min as f32 > 1.3 {
            continue;
        }
        border_row = Some((y, runs));
        break;
    }

    let Some((row, runs)) = border_row else {
        return Vec::new();
    };

    let mut boxes = Vec::with_capacity(3);

    // Pitch from the three centres; every card then shares one width estimate.
    let centres: Vec<f32> = runs
        .iter()
        .map(|(a, b)| *a as f32 + (b - a + 1) as f32 / 2.0)
        .collect();
    let pitch = (centres[2] - centres[0]) / 2.0;
    if pitch <= 0.0 {
        return Vec::new();
    }
    let width = (pitch * CARD_WIDTH_FROM_PITCH).round() as u32;

    for (slot, (a, b)) in runs.iter().enumerate() {
        let run_width = (b - a + 1) as f32;
        let centre_x = *a as f32 + run_width / 2.0;
        let icon_size = ((width as f32 * ICON_SIZE_FRAC).round() as u32).max(8);
        let icon_x = (centre_x - icon_size as f32 / 2.0).max(0.0) as u32;
        let icon_y = row + (width as f32 * ICON_TOP_FROM_RUN).round() as u32;
        let title_y = icon_y + (icon_size as f32 * TITLE_TOP_FROM_ICON).round() as u32;
        let title_h = ((icon_size as f32 * TITLE_HEIGHT_FROM_ICON).round() as u32).max(10);
        boxes.push(CardBox {
            slot,
            centre_x: centre_x as u32,
            width,
            icon_x,
            icon_y,
            icon_size,
            title_x: icon_x.saturating_sub(icon_size / 4),
            title_y,
            title_w: (width - icon_size / 2).min(w),
            title_h,
        });
    }

    // The art must land inside the frame.
    if boxes
        .iter()
        .any(|b| b.icon_y + b.icon_size > h || b.icon_x + b.icon_size > w)
    {
        return Vec::new();
    }

    boxes
}

// ---------------------------------------------------------------
// Icon fingerprints
// ---------------------------------------------------------------

/// Box-averaged RGB thumbnail of a square screen patch.
fn patch_thumb(frame: &Frame, x: u32, y: u32, size: u32) -> Vec<f32> {
    let mut out = vec![0.0f32; THUMB * THUMB * 3];
    let size = size.max(1);
    let x1 = (x + size).min(frame.width);
    let y1 = (y + size).min(frame.height);
    if x1 <= x || y1 <= y {
        return out;
    }
    for ty in 0..THUMB {
        let sy0 = y + (ty as u32 * size) / THUMB as u32;
        let sy1 = (y + ((ty as u32 + 1) * size) / THUMB as u32).clamp(sy0 + 1, y1);
        for tx in 0..THUMB {
            let sx0 = x + (tx as u32 * size) / THUMB as u32;
            let sx1 = (x + ((tx as u32 + 1) * size) / THUMB as u32).clamp(sx0 + 1, x1);
            let mut acc = [0f32; 3];
            let mut n = 0f32;
            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let i = ((sy * frame.width + sx) * 4) as usize;
                    acc[0] += frame.bgra[i + 2] as f32;
                    acc[1] += frame.bgra[i + 1] as f32;
                    acc[2] += frame.bgra[i] as f32;
                    n += 1.0;
                }
            }
            if n > 0.0 {
                let o = (ty * THUMB + tx) * 3;
                out[o] = acc[0] / n;
                out[o + 1] = acc[1] / n;
                out[o + 2] = acc[2] / n;
            }
        }
    }
    out
}

/// Zero-mean, L2-normalised values of `thumb` at the given flat pixel indices.
fn masked_reference(thumb: &[f32], idx: &[u32]) -> Vec<f32> {
    let mut values: Vec<f32> = Vec::with_capacity(idx.len() * 3);
    for &i in idx {
        let base = i as usize * 3;
        values.extend_from_slice(&thumb[base..base + 3]);
    }
    if values.len() < 8 {
        return Vec::new();
    }
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    let mut norm = 0.0f32;
    for v in values.iter_mut() {
        *v -= mean;
        norm += *v * *v;
    }
    let norm = norm.sqrt();
    if norm > 1e-6 {
        for v in values.iter_mut() {
            *v /= norm;
        }
    }
    values
}

struct IconGroup {
    /// Range into `IconIndex::members`.
    members: std::ops::Range<usize>,
    /// Flat pixel indices of the art's opaque area.
    idx: Vec<u32>,
    /// Zero-mean, unit-norm premultiplied RGB at those pixels.
    reference: Vec<f32>,
}

impl IconGroup {
    /// Normalised cross-correlation against a probe thumbnail.
    ///
    /// `reference` is zero-mean, so the probe's own mean does not have to be
    /// subtracted element-wise: `sum((v-m)*r) == sum(v*r)` when `sum(r) == 0`.
    /// That keeps this loop allocation-free.
    fn score(&self, thumb: &[f32]) -> f32 {
        if self.reference.is_empty() || self.idx.is_empty() {
            return -1.0;
        }
        let mut sum = 0.0f32;
        let mut sq = 0.0f32;
        let mut dot = 0.0f32;
        let mut k = 0usize;
        for &i in &self.idx {
            let base = i as usize * 3;
            for c in 0..3 {
                let v = thumb[base + c];
                sum += v;
                sq += v * v;
                dot += v * self.reference[k];
                k += 1;
            }
        }
        let n = (self.idx.len() * 3) as f32;
        let var = sq - sum * sum / n;
        if var <= 1e-6 {
            return -1.0;
        }
        dot / var.sqrt()
    }
}

/// Fingerprints built from one thumbnail per unique piece of card art.
pub struct IconIndex {
    groups: Vec<IconGroup>,
    /// Pool indices, grouped in the same order as `groups`.
    members: Vec<usize>,
}

impl IconIndex {
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Number of augments reachable through this index.
    pub fn augment_count(&self) -> usize {
        self.members.len()
    }

    /// Builds the index from the ARAM Mayhem pool, loading card art from
    /// `art_dir` and downloading anything missing.
    pub async fn build(pool: &[AugmentScanEntry], art_dir: &Path) -> Result<Self, String> {
        let mut by_art: Vec<(String, Vec<usize>)> = Vec::new();
        let mut art_slot: HashMap<String, usize> = HashMap::new();
        for (i, entry) in pool.iter().enumerate() {
            if entry.icon_url.is_empty() {
                continue;
            }
            let slot = *art_slot.entry(entry.icon_url.clone()).or_insert_with(|| {
                by_art.push((entry.icon_url.clone(), Vec::new()));
                by_art.len() - 1
            });
            by_art[slot].1.push(i);
        }
        if by_art.is_empty() {
            return Err("augment pool has no card art".to_string());
        }

        let urls: Vec<String> = by_art.iter().map(|(u, _)| u.clone()).collect();
        let art = fetch_art(urls, art_dir).await;

        let mut groups = Vec::with_capacity(by_art.len());
        let mut members: Vec<usize> = Vec::with_capacity(pool.len());
        for ((url, group_members), bytes) in by_art.into_iter().zip(art.into_iter()) {
            let start = members.len();
            members.extend(group_members.iter().copied());
            let decoded = bytes
                .and_then(|bytes| image::load_from_memory(&bytes).ok())
                .map(|img| img.to_rgba8())
                .ok_or_else(|| format!("card art unavailable: {url}"));

            let (reference, idx) = match decoded {
                Ok(img) => {
                    let small = image::imageops::resize(
                        &img,
                        THUMB as u32,
                        THUMB as u32,
                        image::imageops::FilterType::Triangle,
                    );
                    let mut idx: Vec<u32> = Vec::with_capacity(THUMB * THUMB);
                    let mut premultiplied = vec![0f32; THUMB * THUMB * 3];
                    for (i, px) in small.pixels().enumerate() {
                        let a = px[3] as f32 / 255.0;
                        if px[3] > 96 {
                            idx.push(i as u32);
                        }
                        premultiplied[i * 3] = px[0] as f32 * a;
                        premultiplied[i * 3 + 1] = px[1] as f32 * a;
                        premultiplied[i * 3 + 2] = px[2] as f32 * a;
                    }
                    (masked_reference(&premultiplied, &idx), idx)
                }
                // Keep the group addressable even without art; it never matches.
                Err(err) => {
                    crate::logger::write(crate::logger::Level::Warn, "augment_scan", &err, None);
                    (Vec::new(), Vec::new())
                }
            };

            groups.push(IconGroup {
                members: start..members.len(),
                idx,
                reference,
            });
        }

        if groups.iter().all(|g| g.reference.is_empty()) {
            return Err("no augment card art could be loaded".to_string());
        }
        Ok(Self { groups, members })
    }

    /// Score of every group for one probe thumbnail.
    fn score_patch(&self, thumb: &[f32]) -> Vec<(usize, f32)> {
        self.groups
            .iter()
            .enumerate()
            .map(|(i, group)| (i, group.score(thumb)))
            .collect()
    }

    fn members_of(&self, group: usize) -> &[usize] {
        let range = self.groups[group].members.clone();
        &self.members[range]
    }
}

/// Downloads any card art that is not already cached, with bounded concurrency.
async fn fetch_art(urls: Vec<String>, art_dir: &Path) -> Vec<Option<Vec<u8>>> {
    let _ = tokio::fs::create_dir_all(art_dir).await;
    let mut results: Vec<Option<Vec<u8>>> = vec![None; urls.len()];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    let mut missing: Vec<(usize, String, PathBuf)> = Vec::new();
    for (i, url) in urls.iter().enumerate() {
        let path = art_dir.join(art_cache_name(url));
        match tokio::fs::read(&path).await {
            Ok(bytes) if !bytes.is_empty() => results[i] = Some(bytes),
            _ => missing.push((i, url.clone(), path)),
        }
    }
    if missing.is_empty() {
        return results;
    }

    crate::logger::write(
        crate::logger::Level::Info,
        "augment_scan",
        &format!(
            "Fetching {} augment card art files ({} already cached)",
            missing.len(),
            urls.len() - missing.len()
        ),
        None,
    );

    for chunk in missing.chunks(8) {
        let mut handles = Vec::with_capacity(chunk.len());
        for (i, url, path) in chunk {
            let client = client.clone();
            let url = url.clone();
            let path = path.clone();
            let index = *i;
            handles.push(async move {
                let bytes = match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        resp.bytes().await.ok().map(|b| b.to_vec())
                    }
                    _ => None,
                };
                if let Some(ref b) = bytes {
                    if !b.is_empty() {
                        let _ = tokio::fs::write(&path, b).await;
                    }
                }
                (index, bytes)
            });
        }
        for (i, bytes) in futures_util::future::join_all(handles).await {
            results[i] = bytes.filter(|b| !b.is_empty());
        }
    }
    results
}

fn art_cache_name(url: &str) -> String {
    let hash: u64 = url
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325u64, |acc, b| {
            (acc ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
        });
    format!("{hash:016x}.png")
}

/// Scores one candidate icon position and folds the result into `best`.
fn score_position(
    index: &IconIndex,
    frame: &Frame,
    best: &mut [f32],
    ix: u32,
    iy: u32,
    dx: i32,
    dy: i32,
    size: u32,
) {
    let thumb = patch_thumb(
        frame,
        ix.saturating_add_signed(dx),
        iy.saturating_add_signed(dy),
        size,
    );
    for (group, score) in index.score_patch(&thumb) {
        if score > best[group] {
            best[group] = score;
        }
    }
}

/// Result of a cheap "are the cards up, and which one is highlighted" probe.
pub struct PanelProbe {
    pub frame: Frame,
    pub cards: Vec<CardBox>,
    /// Mean brightness of each card relative to the average of all three.
    /// The game brightens and lifts the card under the cursor / being picked,
    /// so a value well above 1 identifies the selection.
    pub selection: [f32; 3],
    /// Coarse 4x4 luma grid per card: changes when the offered cards change, which
    /// is how a reroll is detected without the panel ever closing.
    pub signature: Vec<f32>,
}

/// Roughly how tall a card is relative to its width (measured on the reference
/// screenshots: 333px tall for a 213px wide card).
const CARD_HEIGHT_FROM_WIDTH: f32 = 1.56;

/// Geometry-only panel probe. No fingerprints, no OCR: just "are three cards
/// there, and is one of them lit up".
pub fn probe_panel(frame: Frame) -> Option<PanelProbe> {
    let cards = detect_cards(&frame);
    if cards.is_empty() {
        return None;
    }
    // Bright runs alone can happen anywhere in a fight. The augment panel also
    // dims the game behind the cards, so the strip just above the card tops is
    // far darker than the card's own top edge. This is what makes the automatic
    // trigger safe to run during normal play.
    if !looks_like_panel(&frame, &cards) {
        return None;
    }
    let selection = selection_scores(&frame, &cards);
    let signature = card_signature(&frame, &cards);
    Some(PanelProbe {
        frame,
        cards,
        selection,
        signature,
    })
}

/// Coarse 4x4 luma grid of each card's art. A reroll swaps the art in place, so
/// comparing this against the previous probe is how the watcher notices that the
/// offered cards changed.
pub fn card_signature(frame: &Frame, cards: &[CardBox]) -> Vec<f32> {
    let mut out = Vec::with_capacity(cards.len() * 16);
    for card in cards.iter().take(3) {
        let (x, y, size) = card.icon_rect();
        for by in 0..4u32 {
            for bx in 0..4u32 {
                let cx0 = x + size * bx / 4;
                let cx1 = (x + size * (bx + 1) / 4).min(frame.width);
                let cy0 = y + size * by / 4;
                let cy1 = (y + size * (by + 1) / 4).min(frame.height);
                let mut sum = 0.0f64;
                let mut n = 0u64;
                let mut py = cy0;
                while py < cy1 {
                    let mut px = cx0;
                    while px < cx1 {
                        let i = ((py * frame.width + px) * 4) as usize;
                        sum += 0.114 * frame.bgra[i] as f64
                            + 0.587 * frame.bgra[i + 1] as f64
                            + 0.299 * frame.bgra[i + 2] as f64;
                        n += 1;
                        px += 2;
                    }
                    py += 2;
                }
                out.push(if n > 0 { (sum / n as f64) as f32 } else { 0.0 });
            }
        }
    }
    out
}

/// True when the offered cards changed (reroll), i.e. the art no longer matches.
///
/// Per-card normalised correlation, not a raw difference: hovering a card brightens
/// it, which must not look like a reroll.
pub fn signature_changed(previous: &[f32], current: &[f32]) -> bool {
    if previous.len() != current.len() || previous.len() % 16 != 0 {
        return !previous.is_empty() || !current.is_empty();
    }
    for card in 0..(previous.len() / 16) {
        let a = &previous[card * 16..(card + 1) * 16];
        let b = &current[card * 16..(card + 1) * 16];
        let ma = a.iter().sum::<f32>() / 16.0;
        let mb = b.iter().sum::<f32>() / 16.0;
        let mut num = 0.0f32;
        let mut da = 0.0f32;
        let mut db = 0.0f32;
        for i in 0..16 {
            let (x, y) = (a[i] - ma, b[i] - mb);
            num += x * y;
            da += x * x;
            db += y * y;
        }
        let denom = (da * db).sqrt();
        // Flat art (or no art) carries no information; do not call that a reroll.
        if denom < 1.0 {
            continue;
        }
        if num / denom < SIGNATURE_MATCH_CORRELATION {
            return true;
        }
    }
    false
}

/// A card whose art correlates below this with the previous probe has been replaced.
const SIGNATURE_MATCH_CORRELATION: f32 = 0.92;

/// The panel's backdrop is dark relative to the bright card edge.
fn looks_like_panel(frame: &Frame, cards: &[CardBox]) -> bool {
    let card = &cards[0];
    let w = card.width as f32;
    let bar_y = card.icon_y as f32 - w * 0.12;
    let above_top = bar_y - w * 0.34;
    if above_top < 0.0 || bar_y <= above_top {
        return false;
    }
    let x_from = (card.centre_x as f32 - w / 2.0).max(0.0) as u32;
    let x_end = ((card.centre_x as f32 + w / 2.0) as u32).min(frame.width);
    if x_end <= x_from {
        return false;
    }

    // Row profiles rather than band means: the bright edge does not span the full
    // card width, so the peak row is the honest reading of "how bright is the bar".
    let row_means = |y_from: f32, y_to: f32| -> Vec<f32> {
        let mut out = Vec::new();
        let mut y = y_from.max(0.0) as u32;
        let y_end = (y_to as u32).min(frame.height);
        while y < y_end {
            let mut sum = 0.0f64;
            let mut count = 0u64;
            let mut x = x_from;
            while x < x_end {
                let idx = ((y * frame.width + x) * 4) as usize;
                sum += 0.114 * frame.bgra[idx] as f64
                    + 0.587 * frame.bgra[idx + 1] as f64
                    + 0.299 * frame.bgra[idx + 2] as f64;
                count += 1;
                x += 3;
            }
            if count > 0 {
                out.push((sum / count as f64) as f32);
            }
            y += 1;
        }
        out
    };

    let bar_rows = row_means(bar_y, bar_y + 4.0);
    let above_rows = row_means(above_top, bar_y - 3.0);
    if bar_rows.is_empty() || above_rows.is_empty() {
        return false;
    }
    let bar = bar_rows.iter().cloned().fold(0.0f32, f32::max);
    let above = above_rows.iter().sum::<f32>() / above_rows.len() as f32;
    bar > 100.0 && above < bar * 0.70
}

/// Relative brightness of each card. Cheap: samples every 4th pixel.
pub fn selection_scores(frame: &Frame, cards: &[CardBox]) -> [f32; 3] {
    let mut means = [0.0f32; 3];
    for (i, card) in cards.iter().take(3).enumerate() {
        let w = card.width as f32;
        let x0 = (card.centre_x as f32 - w / 2.0).max(0.0) as u32;
        let y0 = (card.icon_y as f32 - w * 0.12).max(0.0) as u32;
        let cw = card.width.min(frame.width.saturating_sub(x0));
        let ch = ((w * CARD_HEIGHT_FROM_WIDTH) as u32).min(frame.height.saturating_sub(y0));
        if cw < 8 || ch < 8 {
            continue;
        }
        let mut sum = 0.0f64;
        let mut count = 0u64;
        let mut y = y0;
        while y + 1 < y0 + ch {
            let mut x = x0;
            while x + 1 < x0 + cw {
                let idx = ((y * frame.width + x) * 4) as usize;
                let (b, g, r) = (
                    frame.bgra[idx] as f64,
                    frame.bgra[idx + 1] as f64,
                    frame.bgra[idx + 2] as f64,
                );
                sum += 0.114 * b + 0.587 * g + 0.299 * r;
                count += 1;
                x += 4;
            }
            y += 4;
        }
        if count > 0 {
            means[i] = (sum / count as f64) as f32;
        }
    }

    let present = means.iter().filter(|m| **m > 0.0).count();
    let avg = if present > 0 {
        means.iter().filter(|m| **m > 0.0).sum::<f32>() / present as f32
    } else {
        0.0
    };
    if avg <= 1.0 {
        return [1.0; 3];
    }
    [
        (means[0] / avg).clamp(0.0, 4.0),
        (means[1] / avg).clamp(0.0, 4.0),
        (means[2] / avg).clamp(0.0, 4.0),
    ]
}

/// Which slot was picked, judged by how much each card brightened **relative to
/// its own first-probe brightness**.
///
/// Comparing cards against each other does not work: the middle card of the panel
/// is rendered brighter than its neighbours regardless of art (measured 1.16x on
/// both reference screenshots), so an absolute comparison calls every middle card
/// a pick. A rise above a card's own baseline only happens on hover/selection.
pub fn picked_from_baseline(baseline: [f32; 3], current: [f32; 3]) -> Option<usize> {
    if baseline.iter().any(|b| *b <= 0.01) {
        return None;
    }
    let mut deltas = [0.0f32; 3];
    for i in 0..3 {
        deltas[i] = current[i] / baseline[i];
    }
    let mut order: Vec<(usize, f32)> = deltas.iter().copied().enumerate().collect();
    order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let (best_slot, best) = order[0];
    let second = order[1].1;
    if best >= HIGHLIGHT_MIN_RATIO && second <= HIGHLIGHT_TIE_RATIO {
        Some(best_slot)
    } else {
        None
    }
}

/// A card that brightens this much over its own baseline is being selected...
const HIGHLIGHT_MIN_RATIO: f32 = 1.12;
/// ...while the other two stay this close to their own baselines.
const HIGHLIGHT_TIE_RATIO: f32 = 1.06;

// ---------------------------------------------------------------
// Title OCR
// ---------------------------------------------------------------

/// Windows OCR over one region of the frame, ready for the engine.
#[cfg(windows)]
fn ocr_rect(
    engine: &windows::Media::Ocr::OcrEngine,
    frame: &Frame,
    rect: (u32, u32, u32, u32),
) -> String {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Storage::Streams::DataWriter;

    let (x, y, w, h) = rect;
    let crop = frame.crop_rgba(x, y, w, h);
    // Small crops are upscaled so OCR sees glyphs around 20 px tall. Body text
    // is much smaller than titles: on a 1280x720 capture the description glyphs
    // are ~11 px, which Windows OCR misreads as garbage, so the threshold has to
    // cover description rects too, not just title strips.
    let factor = if h < 140 { 2 } else { 1 };
    let scaled = image::imageops::resize(
        &crop,
        w * factor,
        h * factor,
        image::imageops::FilterType::Lanczos3,
    );

    let mut bgra = Vec::with_capacity((scaled.width() * scaled.height() * 4) as usize);
    for px in scaled.pixels() {
        bgra.extend_from_slice(&[px[2], px[1], px[0], 255]);
    }

    let Ok(writer) = DataWriter::new() else {
        return String::new();
    };
    if writer.WriteBytes(&bgra).is_err() {
        return String::new();
    }
    let Ok(buffer) = writer.DetachBuffer() else {
        return String::new();
    };
    let Ok(bitmap) = SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        scaled.width() as i32,
        scaled.height() as i32,
    ) else {
        return String::new();
    };
    let Ok(result) = engine.RecognizeAsync(&bitmap).and_then(|op| op.get()) else {
        return String::new();
    };

    let mut text = String::new();
    if let Ok(lines) = result.Lines() {
        let count = lines.Size().unwrap_or(0);
        for i in 0..count {
            if let Ok(line) = lines.GetAt(i) {
                if let Ok(t) = line.Text() {
                    let t = t.to_string();
                    let t = t.trim();
                    if !t.is_empty() {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(t);
                    }
                }
            }
        }
    }
    text
}

/// Windows OCR over the three title strips, one string per card in slot order.
/// Only called when the art match is ambiguous.
#[cfg(windows)]
async fn ocr_titles(frame: &Frame, cards: &[CardBox]) -> Result<Vec<String>, String> {
    use windows::Media::Ocr::OcrEngine;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| format!("OCR engine unavailable: {e}"))?;
    Ok(cards
        .iter()
        .map(|card| ocr_rect(&engine, frame, card.title_rect()))
        .collect())
}

/// Body text of each card. Used only for cards whose art is shared between
/// augments, where the body text is the only distinguishing evidence.
#[cfg(windows)]
async fn ocr_descriptions(frame: &Frame, cards: &[CardBox]) -> Result<Vec<String>, String> {
    use windows::Media::Ocr::OcrEngine;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| format!("OCR engine unavailable: {e}"))?;
    Ok(cards
        .iter()
        .map(|card| ocr_rect(&engine, frame, card.description_rect()))
        .collect())
}

#[cfg(not(windows))]
async fn ocr_titles(_frame: &Frame, _cards: &[CardBox]) -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

#[cfg(not(windows))]
async fn ocr_descriptions(_frame: &Frame, _cards: &[CardBox]) -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

// ---------------------------------------------------------------
// Name matching
// ---------------------------------------------------------------

fn normalize_name(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for i in 0..a.len() {
        curr[0] = i + 1;
        for j in 0..b.len() {
            let cost = usize::from(a[i] != b[j]);
            curr[j + 1] = (curr[j + 1] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        prev.copy_from_slice(&curr);
    }
    prev[b.len()]
}

/// 0..1 similarity between OCR'd title text and an augment name.
fn name_similarity(ocr_text: &str, name: &str) -> f64 {
    let a = normalize_name(ocr_text);
    let b = normalize_name(name);
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    if a == b {
        return 1.0;
    }
    if a.contains(&b) || b.contains(&a) {
        let ratio = a.len().min(b.len()) as f64 / a.len().max(b.len()) as f64;
        return 0.60 + 0.38 * ratio;
    }
    let max_len = a.chars().count().max(b.chars().count()) as f64;
    if max_len == 0.0 {
        return 0.0;
    }
    1.0 - levenshtein(&a, &b) as f64 / max_len
}

/// Words that say nothing about which augment a card is. The card text repeats
/// them all ("enemy champions", "grants you"), and the game also substitutes the
/// champion's spell name for `{{SpellName}}`, so those words are noise here too.
const DESCRIPTION_STOPWORDS: &[&str] = &[
    "your", "with", "when", "that", "this", "from", "they", "have", "will", "them", "their",
    "which", "been", "each", "into", "than", "then", "also", "more", "most", "very", "just",
    "only", "gain", "grants", "grant", "gains", "enemy", "enemies", "champion", "champions",
    "damage", "dealt", "using", "while", "after", "before", "within", "against", "other",
    "some", "every", "ability", "abilities", "spell", "spells", "attack", "attacks",
    "percent", "stack", "stacks", "that", "these", "those", "while", "would", "should",
];

fn description_tokens(text: &str) -> Vec<String> {
    let mut out: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() >= 4)
        .filter(|w| !DESCRIPTION_STOPWORDS.contains(&w.as_str()))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// How much of the augment's own card text appears in what OCR read off the card.
///
/// Divided by the shorter of the two token sets: the game substitutes the
/// champion's spell name into the text, so the read side always carries extra
/// words that the stringtable description cannot contain.
pub fn description_similarity(ocr_text: &str, description: &str) -> f64 {
    let read = description_tokens(ocr_text);
    let known = description_tokens(description);
    if read.is_empty() || known.is_empty() {
        return 0.0;
    }
    let hits = known.iter().filter(|w| read.contains(w)).count() as f64;
    (hits / read.len().min(known.len()) as f64).min(1.0)
}

/// Minimum body-text similarity to name a shared-art card from its text.
const DESCRIPTION_MIN_SCORE: f64 = 0.45;
/// ... and how far ahead of the runner-up it must be.
const DESCRIPTION_MARGIN: f64 = 0.12;
/// Fewer words than this read off the card is not evidence of anything.
const DESCRIPTION_MIN_TOKENS: usize = 3;

// ---------------------------------------------------------------
// Scanner
// ---------------------------------------------------------------

/// A card the scanner is confident about.
#[derive(Debug, Clone)]
pub struct IdentifiedCard {
    pub slot: usize,
    pub entry: AugmentScanEntry,
    pub icon_score: f32,
    /// `None` when the art alone was decisive.
    pub title_score: Option<f64>,
    /// The art matched several augments and the title could not separate them.
    pub ambiguous: bool,
}

#[derive(Debug, Clone)]
pub enum ScanError {
    /// No card borders found: the selection screen is not open.
    NoCards,
    /// The capture came back uniformly black (BitBlt failed on a DX window).
    BlackFrame,
    /// The augment pool or its card art is not available.
    Index(String),
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanError::NoCards => write!(
                f,
                "No augment cards on screen. Open the augment selection and press F8."
            ),
            ScanError::BlackFrame => write!(
                f,
                "Screen capture came back empty. Try borderless/windowed game mode."
            ),
            ScanError::Index(msg) => write!(f, "Augment data unavailable: {msg}"),
        }
    }
}

impl std::error::Error for ScanError {}

/// A pool plus its card-art fingerprints. Built once per patch and reused.
pub struct Scanner {
    pool: Vec<AugmentScanEntry>,
    index: IconIndex,
}

impl Scanner {
    pub async fn load() -> Result<Self, ScanError> {
        let pool = get_aram_augment_pool().await;
        if pool.is_empty() {
            return Err(ScanError::Index(
                "ARAM augment pool is empty (CDragon data unavailable)".to_string(),
            ));
        }
        let art_dir = crate::services::asset_cache::get_cache_dir().join("augment_art");
        let index = IconIndex::build(&pool, &art_dir)
            .await
            .map_err(ScanError::Index)?;
        crate::logger::write(
            crate::logger::Level::Info,
            "augment_scan",
            &format!(
                "Scanner ready: {} augments, {} distinct card arts",
                pool.len(),
                index.len()
            ),
            None,
        );
        Ok(Self { pool, index })
    }

    pub fn pool(&self) -> &[AugmentScanEntry] {
        &self.pool
    }

    pub fn index(&self) -> &IconIndex {
        &self.index
    }

    /// Identifies the offered cards in a captured frame.
/// Best candidate by body text, if one is clearly ahead and enough was read.
fn best_description_match(
    pool: &[AugmentScanEntry],
    candidates: &[usize],
    description: &str,
) -> Option<(usize, f64)> {
    if description_tokens(description).len() < DESCRIPTION_MIN_TOKENS {
        return None;
    }
    let mut scored: Vec<(usize, f64)> = candidates
        .iter()
        .map(|&c| (c, description_similarity(description, &pool[c].description)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let (best, score) = *scored.first()?;
    let second = scored.get(1).map(|(_, s)| *s).unwrap_or(0.0);
    if score >= DESCRIPTION_MIN_SCORE && score - second >= DESCRIPTION_MARGIN {
        Some((best, score))
    } else {
        None
    }
}

    /// Pick the augment for one card from its art candidates plus the title read.
    ///
    /// Returns `(chosen, title_score, ambiguous)`. The important rule: when the
    /// art is *shared* between several augments (Riot ships one generic icon per
    /// rarity for the "ability" augments), the title is the only evidence, so a
    /// title that matches none of them makes the card unconfirmed instead of
    /// overruling the art with some other augment that merely sounds similar.
    fn choose_entry(
        pool: &[AugmentScanEntry],
        candidates: &[usize],
        top_score: f32,
        art_is_unique: bool,
        title: &str,
        description: &str,
    ) -> (usize, Option<f64>, bool) {
        let mut chosen = candidates[0];
        let mut title_score: Option<f64> = None;
        let mut ambiguous = false;

        if candidates.len() == 1 && top_score >= CONFIDENT_ICON_SCORE && art_is_unique {
            return (chosen, None, false);
        }

        // Shared art: the body text is the only evidence that names the augment
        // rather than its class. Read it whenever the card is not art-unique.
        if !art_is_unique {
            if let Some((idx, score)) = Self::best_description_match(pool, candidates, description) {
                return (idx, Some(score), false);
            }
        }

        if title.is_empty() {
            return (chosen, None, candidates.len() > 1 || !art_is_unique);
        }

        let mut best = (candidates[0], 0.0f64);
        for &cand in candidates {
            let sim = name_similarity(title, &pool[cand].name);
            if sim > best.1 {
                best = (cand, sim);
            }
        }
        if best.1 >= MIN_TITLE_SCORE {
            return (best.0, Some(best.1), false);
        }

        if art_is_unique && top_score < CONFIDENT_ICON_SCORE {
            // Weak, unique art and a title matching none of its candidates: only
            // a strong read may replace the art.
            let mut over: Option<(usize, f64)> = None;
            for (idx, entry) in pool.iter().enumerate() {
                let sim = name_similarity(title, &entry.name);
                if over.map(|(_, s)| sim > s).unwrap_or(true) {
                    over = Some((idx, sim));
                }
            }
            if let Some((idx, sim)) = over {
                if sim >= STRONG_OVERRIDE_TITLE_SCORE {
                    return (idx, Some(sim), false);
                }
            }
        }

        chosen = best.0;
        title_score = Some(best.1);
        ambiguous = true;
        (chosen, title_score, ambiguous)
    }

    pub async fn identify(&self, frame: &Frame) -> Result<Vec<IdentifiedCard>, ScanError> {
        let started = std::time::Instant::now();
        if frame.mean_luma() < 2.0 {
            return Err(ScanError::BlackFrame);
        }
        let cards = detect_cards(frame);
        if cards.is_empty() {
            return Err(ScanError::NoCards);
        }

        // Art match per slot. The geometry is derived from the card's own top
        // border, so the nominal position is nearly always right; a small grid
        // search is only used when the nominal position scores poorly.
        let mut per_slot: Vec<Vec<(usize, f32)>> = Vec::with_capacity(cards.len());
        for card in &cards {
            let (ix, iy, size) = card.icon_rect();
            let mut best = vec![-1.0f32; self.index.len()];

            score_position(&self.index, frame, &mut best, ix, iy, 0, 0, size);
            let nominal = best.iter().cloned().fold(-1.0f32, f32::max);
            if nominal < CONFIDENT_ICON_SCORE {
                for (scale, step) in [(0.97f32, 2i32), (1.03, 2), (1.0, 4)] {
                    let s = ((size as f32 * scale).round() as u32).max(8);
                    for dy in [-step, 0, step] {
                        for dx in [-step, 0, step] {
                            if dx == 0 && dy == 0 && (scale - 1.0).abs() < 1e-6 {
                                continue;
                            }
                            score_position(&self.index, frame, &mut best, ix, iy, dx, dy, s);
                        }
                    }
                }
            }

            let mut scored: Vec<(usize, f32)> = best.into_iter().enumerate().collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            per_slot.push(scored);
        }

        // Candidate set per slot from the art alone.
        let mut candidate_sets: Vec<Vec<usize>> = Vec::with_capacity(per_slot.len());
        for scores in &per_slot {
            let mut candidates: Vec<usize> = Vec::new();
            if let Some(&(_, top_score)) = scores.first() {
                if top_score >= MIN_ICON_SCORE {
                    for (group, score) in scores {
                        if *score < top_score - ICON_TIE_EPSILON {
                            break;
                        }
                        candidates.extend_from_slice(self.index.members_of(*group));
                    }
                }
            }
            // Dedupe while keeping score order: the first entry is the best art match.
            let mut seen = std::collections::HashSet::new();
            candidates.retain(|c| seen.insert(*c));
            candidate_sets.push(candidates);
        }

        let needs_title: Vec<bool> = per_slot
            .iter()
            .zip(candidate_sets.iter())
            .map(|(scores, candidates)| match scores.first() {
                Some(&(_, top)) if top >= MIN_ICON_SCORE => {
                    candidates.len() > 1 || top < CONFIDENT_ICON_SCORE
                }
                _ => false,
            })
            .collect();

        let titles = if needs_title.iter().any(|n| *n) {
            ocr_titles(frame, &cards).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        // Body text is the only thing that distinguishes augments sharing one
        // placeholder icon, so read it whenever any card's art is not unique.
        let shared_art = per_slot.iter().any(|scores| {
            !matches!(
                scores.first(),
                Some(&(group, _)) if self.index.members_of(group).len() == 1
            )
        });
        let descriptions = if shared_art {
            ocr_descriptions(frame, &cards).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut out = Vec::new();
        for (i, card) in cards.iter().enumerate() {
            let top_score = match per_slot[i].first() {
                Some(&(_, score)) => score,
                None => continue,
            };
            if top_score < MIN_ICON_SCORE {
                continue;
            }
            let candidates = &candidate_sets[i];
            if candidates.is_empty() {
                continue;
            }

            // Riot reuses one icon across a whole class of "ability" augments:
            // 6 silver (Siphon, Bolstered, It's Go Time, Stay Resolute, Trusty
            // Weapon, Adaptive Ward), 14 gold, 6 prismatic. Their art names the
            // class, never the augment.
            let art_is_unique = matches!(
                per_slot[i].first(),
                Some(&(group, _)) if self.index.members_of(group).len() == 1
            );
            let title = titles
                .get(i)
                .map(|s| s.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let description = descriptions.get(i).map(|s| s.as_str()).unwrap_or("");
            let (chosen, title_score, ambiguous) = Self::choose_entry(
                &self.pool,
                candidates,
                top_score,
                art_is_unique,
                &title,
                description,
            );

            if ambiguous || top_score < CONFIDENT_ICON_SCORE || title_score.map_or(false, |s| s < 0.6) {
                crate::logger::write(
                    crate::logger::Level::Info,
                    "augment_scan",
                    &format!(
                        "card {i}: art_top={top_score:.2} art_unique={art_is_unique} shorts={} title={title:?} chose={} amb={ambiguous}",
                        candidates.len(),
                        self.pool[chosen].name
                    ),
                    None,
                );
            }

            out.push(IdentifiedCard {
                slot: card.slot,
                entry: self.pool[chosen].clone(),
                icon_score: top_score,
                title_score,
                ambiguous,
            });
        }

        if out.is_empty() {
            return Err(ScanError::NoCards);
        }
        crate::logger::write(
            crate::logger::Level::Info,
            "augment_scan",
            &format!(
                "identified {} cards in {}ms: {}",
                out.len(),
                started.elapsed().as_millis(),
                out.iter()
                    .map(|c| format!(
                        "#{}={} ({:.2}{}{})",
                        c.slot,
                        c.entry.name,
                        c.icon_score,
                        c.title_score
                            .map(|t| format!(", t{t:.2}"))
                            .unwrap_or_default(),
                        if c.ambiguous { ", ambiguous" } else { "" }
                    ))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
            None,
        );
        Ok(out)
    }
}

// ---------------------------------------------------------------
// Process-wide scanner (built once, reused by every scan)
// ---------------------------------------------------------------

lazy_static::lazy_static! {
    static ref SCANNER: tokio::sync::RwLock<Option<Arc<Scanner>>> = tokio::sync::RwLock::new(None);
}

/// Returns the shared scanner, building it on first use.
pub async fn scanner() -> Result<Arc<Scanner>, ScanError> {
    {
        let read = SCANNER.read().await;
        if let Some(scanner) = read.as_ref() {
            return Ok(scanner.clone());
        }
    }
    let mut write = SCANNER.write().await;
    if let Some(scanner) = write.as_ref() {
        return Ok(scanner.clone());
    }
    let built = Arc::new(Scanner::load().await?);
    *write = Some(built.clone());
    Ok(built)
}

/// Warms the scanner in the background so the first F8 is instant.
pub fn warm_up() {
    tauri::async_runtime::spawn(async move {
        let started = std::time::Instant::now();
        match scanner().await {
            Ok(s) => crate::logger::write(
                crate::logger::Level::Info,
                "augment_scan",
                &format!(
                    "Scanner warmed in {}ms ({} augments, {} card arts)",
                    started.elapsed().as_millis(),
                    s.pool().len(),
                    s.index().len()
                ),
                None,
            ),
            Err(err) => crate::logger::write(
                crate::logger::Level::Warn,
                "augment_scan",
                &format!("Scanner warm-up failed: {err}"),
                None,
            ),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_of(name: &str) -> Option<Frame> {
        let path = PathBuf::from(r"C:\Riot Games\League of Legends\Screenshots").join(name);
        let img = image::open(&path).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        Some(Frame::from_rgba(img.as_raw(), w, h))
    }

    #[test]
    fn name_similarity_handles_common_ocr_damage() {
        assert!(name_similarity("Symphony of War", "Symphony of War") > 0.99);
        assert!(name_similarity("Symphony of War", "SymphonyofWar") > 0.99);
        assert!(name_similarity("From Downwhirl", "From Downtown") > 0.6);
        assert!(name_similarity("Symphony of War", "Clown College") < 0.4);
        assert!(name_similarity("", "Eureka") < 0.01);
    }

    #[test]
    fn detects_three_cards_on_the_reference_screenshots() {
        // (file, expected card width, expected icon centre x, expected icon top)
        let cases: [(&str, u32, [u32; 3], u32); 2] = [
            ("0-Screen76.png", 213, [342, 587, 832], 154),
            ("Screen15.png", 318, [519, 887, 1254], 231),
        ];
        for (name, expected_w, expected_icon_x, expected_icon_y) in cases {
            let Some(frame) = frame_of(name) else {
                eprintln!("skipping {name}: not found");
                continue;
            };
            let cards = detect_cards(&frame);
            assert_eq!(cards.len(), 3, "{name}: expected 3 cards");
            for (i, card) in cards.iter().enumerate() {
                assert_eq!(card.slot, i);
                assert!(
                    (card.width as i64 - expected_w as i64).abs() <= 8,
                    "{name}: card {i} width {} != {expected_w}",
                    card.width
                );
                let (x, y, size) = card.icon_rect();
                assert!(
                    (x as i64 - expected_icon_x[i] as i64).abs() <= 5,
                    "{name}: card {i} icon x {x} != {}",
                    expected_icon_x[i]
                );
                assert!(
                    (y as i64 - expected_icon_y as i64).abs() <= 4,
                    "{name}: card {i} icon y {y} != {expected_icon_y}"
                );
                assert!(
                    (size as i64 - (expected_w / 2) as i64).abs() <= 5,
                    "{name}: card {i} icon size {size}"
                );
            }
        }
    }

    #[test]
    fn no_cards_on_a_plain_frame() {
        let frame = Frame::from_rgba(&vec![30u8; 800 * 450 * 4], 800, 450);
        assert!(detect_cards(&frame).is_empty());

        let black = Frame::from_rgba(&vec![0u8; 800 * 450 * 4], 800, 450);
        assert!(detect_cards(&black).is_empty());
    }

    #[test]
    fn auto_probe_sees_the_real_panel() {
        for name in ["0-Screen76.png", "Screen15.png"] {
            let Some(frame) = frame_of(name) else {
                return; // screenshots not present on this machine
            };
            let probe = probe_panel(frame).expect("panel probe should see the cards");
            assert_eq!(probe.cards.len(), 3, "{name} should show three cards");
            assert!(
                probe.selection.iter().all(|s| *s > 0.5),
                "{name} selection ratios should be plausible: {:?}",
                probe.selection
            );
        }
    }

    /// Three bright bars alone are not an augment panel: the backdrop has to be
    /// dimmed, otherwise the watcher would fire mid-fight.
    #[test]
    fn auto_probe_ignores_bright_bars_on_a_bright_background() {
        fn frame_with_backdrop(luma: u8) -> Frame {
            let (w, h) = (1280u32, 720u32);
            let mut bgra = vec![luma; (w * h * 4) as usize];
            for px in bgra.chunks_exact_mut(4) {
                px[3] = 255;
            }
            for x0 in [200u32, 560, 920] {
                for y in 150..153u32 {
                    for x in x0..x0 + 150 {
                        let i = ((y * w + x) * 4) as usize;
                        bgra[i..i + 4].copy_from_slice(&[245, 245, 245, 255]);
                    }
                }
            }
            Frame {
                bgra,
                width: w,
                height: h,
            }
        }

        let dim = frame_with_backdrop(25);
        assert!(probe_panel(dim).is_some(), "dim backdrop is a panel");
        let bright = frame_with_backdrop(200);
        assert!(
            probe_panel(bright).is_none(),
            "bright backdrop is gameplay, not the panel"
        );
    }

    #[test]
    fn reroll_detection_ignores_brightness_but_sees_new_art() {
        let base: Vec<f32> = (0..48).map(|i| 20.0 + (i as f32 * 7.0) % 90.0).collect();
        // Same cards, hover brightened one of them: not a reroll.
        let mut brighter = base.clone();
        for v in brighter[16..32].iter_mut() {
            *v *= 1.15;
        }
        assert!(!signature_changed(&base, &base));
        assert!(!signature_changed(&base, &brighter));

        // The middle card was rerolled: its art pattern is different.
        let mut rerolled = base.clone();
        for (i, v) in rerolled[16..32].iter_mut().enumerate() {
            *v = 30.0 + ((16 - i) as f32 * 5.5) % 70.0;
        }
        assert!(signature_changed(&base, &rerolled));
    }

    fn entry(name: &str, key: &str) -> AugmentScanEntry {
        AugmentScanEntry {
            key: key.to_string(),
            name: name.to_string(),
            name_id: name.replace(' ', ""),
            icon_url: String::new(),
            rarity: Some("silver".to_string()),
            categories: Vec::new(),
            description: String::new(),
        }
    }

    /// The reported bug: a "Siphon" card was named "Spin Me Right Round".
    /// Siphon's art is the shared generic silver ability icon, so its own art
    /// group holds six augments; a misread title must not escape that group.
    #[test]
    fn shared_art_is_never_overridden_by_a_distant_title() {
        let pool = vec![
            entry("Siphon", "2073"),
            entry("Bolstered", "2001"),
            entry("It's Go Time", "2002"),
            entry("Stay Resolute", "2003"),
            entry("Trusty Weapon", "2004"),
            entry("Adaptive Ward", "2005"),
            entry("Spin Me Right Round", "1386"),
        ];
        let shared: Vec<usize> = (0..6).collect();

        // Clean title: the group member is picked.
        let (chosen, score, ambiguous) =
            Scanner::choose_entry(&pool, &shared, 0.97, false, "Siphon", "");
        assert_eq!(pool[chosen].name, "Siphon");
        assert!(score.unwrap() > 0.9);
        assert!(!ambiguous);

        // Garbled title: stay inside the shared group and say it is unconfirmed.
        // "Spinning" is what a bad read of the shared icon's glyphs produced;
        // the old code let it jump to another augment entirely.
        let (chosen, _, ambiguous) =
            Scanner::choose_entry(&pool, &shared, 0.97, false, "Spinning Wheel", "");
        assert!(
            chosen < 6,
            "must stay inside the shared-art group, got {}",
            pool[chosen].name
        );
        assert!(ambiguous, "an unreadable shared-art card is unconfirmed");

        // Empty title on a shared group: unconfirmed, never a confident name.
        let (_, _, ambiguous) = Scanner::choose_entry(&pool, &shared, 0.97, false, "", "");
        assert!(ambiguous);
    }

    /// Unique art with a weak score may still be rescued by a strong title read.
    #[test]
    fn a_strong_title_rescues_a_weak_unique_art_match() {
        let pool = vec![entry("Siphon", "2073"), entry("Spin Me Right Round", "1386")];
        let (chosen, _, ambiguous) =
            Scanner::choose_entry(&pool, &[0], 0.64, true, "Spin Me Right Round", "");
        assert_eq!(pool[chosen].name, "Spin Me Right Round");
        assert!(!ambiguous);

        // A merely similar title must not replace weak art either.
        let (chosen, _, _) = Scanner::choose_entry(&pool, &[0], 0.64, true, "Spin", "");
        assert_eq!(pool[chosen].name, "Siphon");
    }

    #[test]
    fn pick_detection_needs_a_card_to_brighten_over_its_own_baseline() {
        // The middle card of an untouched panel is 1.16x brighter than the others.
        let baseline = [0.92, 1.16, 0.92];
        // Nothing moved: not a pick, even though the middle card is the brightest.
        assert_eq!(picked_from_baseline(baseline, [0.92, 1.16, 0.92]), None);
        // The left card lights up: that is the pick.
        assert_eq!(picked_from_baseline(baseline, [1.32, 1.05, 0.88]), Some(0));
        // The right card lights up.
        assert_eq!(picked_from_baseline(baseline, [0.88, 1.04, 1.30]), Some(2));
        // Two cards lighting up (cursor swept across): ambiguous, do not guess.
        assert_eq!(picked_from_baseline(baseline, [1.30, 1.28, 0.90]), None);
        // Everything dims as the panel closes: not a pick.
        assert_eq!(picked_from_baseline(baseline, [0.60, 0.75, 0.60]), None);
    }

    #[test]
    fn selection_scores_average_to_one_on_a_uniform_frame() {
        let w = 220u32;
        let h = 300u32;
        let frame = Frame {
            bgra: vec![120u8; (w * h * 4) as usize],
            width: w,
            height: h,
        };
        let cards = detect_cards(&frame);
        // A flat frame has no bright bar, so no cards; the score helper must not
        // divide by zero either way.
        let scores = selection_scores(&frame, &cards);
        assert!(scores.iter().all(|s| (*s - 1.0).abs() < 0.5));
    }

    #[test]
    fn icon_rect_tracks_the_card_width() {
        let card = CardBox {
            slot: 0,
            centre_x: 394,
            width: 213,
            icon_x: 341,
            icon_y: 154,
            icon_size: 107,
            title_x: 314,
            title_y: 275,
            title_w: 160,
            title_h: 43,
        };
        assert_eq!(card.icon_rect(), (341, 154, 107));
        assert_eq!(card.title_rect(), (314, 275, 160, 43));
    }

    #[test]
    fn art_cache_names_are_stable_and_unique() {
        let a = art_cache_name("https://x/a.png");
        assert_eq!(a, art_cache_name("https://x/a.png"));
        assert_ne!(a, art_cache_name("https://x/b.png"));
    }

    /// The real thing: pool + card art + geometry + OCR on the reference
    /// screenshots. Skips (rather than fails) when the CDragon data cannot be
    /// fetched, so it stays usable offline.
    #[tokio::test]
    async fn identifies_reference_cards_end_to_end() {
        let scanner = match super::scanner().await {
            Ok(scanner) => scanner,
            Err(err) => {
                eprintln!("skipping end-to-end scan: {err}");
                return;
            }
        };

        let cases: [(&str, [&str; 3]); 2] = [
            (
                "0-Screen76.png",
                ["Symphony of War", "Clown College", "Mystic Punch"],
            ),
            (
                "Screen15.png",
                ["Skilled Sniper", "Tooth Fairy", "From Downtown"],
            ),
        ];

        for (file, expected) in cases {
            let Some(frame) = frame_of(file) else {
                eprintln!("skipping {file}: not found");
                continue;
            };
            let started = std::time::Instant::now();
            let cards = scanner
                .identify(&frame)
                .await
                .unwrap_or_else(|e| panic!("{file}: {e}"));
            assert_eq!(cards.len(), 3, "{file}: expected 3 cards");
            let names: Vec<&str> = cards.iter().map(|c| c.entry.name.as_str()).collect();
            assert_eq!(names, expected.to_vec(), "{file}");
            println!("{file} identified in {:?}", started.elapsed());
        }
    }
}
