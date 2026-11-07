//! Turns scored cards into draw-ready rows for the native overlay.
//! Pure: no Win32, no IO, so it can be unit-tested on any platform.

use serde::{Deserialize, Serialize};

use crate::models::history::OverlayAugmentCard;

pub const ACCENT_PICK: [u8; 3] = [16, 185, 129];
pub const ACCENT_REROLL: [u8; 3] = [245, 158, 11];
pub const ACCENT_SKIP: [u8; 3] = [148, 163, 184];
pub const ACCENT_IDLE: [u8; 3] = [6, 182, 212];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OverlayCardRow {
    pub slot: usize,
    pub accent: [u8; 3],
    pub header: String,
    pub name: String,
    pub stats: String,
    pub footer: String,
}

fn slot_label(slot: usize) -> &'static str {
    match slot {
        0 => "LEFT",
        1 => "MID",
        _ => "RIGHT",
    }
}

fn verdict_label_and_accent(decision: Option<&str>) -> (&'static str, [u8; 3]) {
    match decision.unwrap_or("") {
        "pick" => ("PICK", ACCENT_PICK),
        "reroll" => ("REROLL", ACCENT_REROLL),
        "skip" => ("SKIP", ACCENT_SKIP),
        _ => ("WAIT", ACCENT_SKIP),
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let kept: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{kept}…")
}

pub fn compact_games(games: usize) -> String {
    if games >= 1000 {
        format!("{:.1}k", games as f64 / 1000.0)
    } else {
        games.to_string()
    }
}

/// Hotkey that records a pick for a card slot.
pub fn pick_key_hint(slot: usize) -> &'static str {
    match slot {
        0 => "F9",
        1 => "F10",
        _ => "F11",
    }
}

/// One row per offered card, in screen order (LEFT, MID, RIGHT).
pub fn build_rows(cards: &[OverlayAugmentCard], max_name_chars: usize) -> Vec<OverlayCardRow> {
    let mut ordered: Vec<&OverlayAugmentCard> = cards.iter().take(3).collect();
    ordered.sort_by_key(|card| card.slot);

    ordered
        .iter()
        .map(|card| {
            let (verdict, accent) = verdict_label_and_accent(card.decision.as_deref());
            let rank = card.rank.unwrap_or(0);
            let key_hint = pick_key_hint(card.slot);
            let header = if rank > 0 {
                format!("{}. {}  •  {}  [{key_hint}]", rank, slot_label(card.slot), verdict)
            } else {
                format!("{}  •  {}  [{key_hint}]", slot_label(card.slot), verdict)
            };
            let stats = if card.unconfirmed {
                // Art is shared between augments and the title was unreadable,
                // so the name (and therefore any stat) is a guess. Say so.
                "unread card — F9/F10/F11 to set".to_string()
            } else if card.pending {
                "reading stats…".to_string()
            } else if card.games == 0 {
                "no sample yet".to_string()
            } else {
                format!(
                    "{:.1}% WR   {:+.1}%   {}",
                    card.win_rate * 100.0,
                    card.wpa * 100.0,
                    compact_games(card.games)
                )
            };
            let footer = card
                .combos
                .first()
                .map(|combo| {
                    format!(
                        "SYN: {} {:+.1}%",
                        truncate_chars(&combo.label, 22),
                        combo.wpa * 100.0
                    )
                })
                .unwrap_or_default();

            OverlayCardRow {
                slot: card.slot,
                accent,
                header,
                name: if card.unconfirmed {
                    truncate_chars(&format!("{} ?", card.name), max_name_chars)
                } else {
                    truncate_chars(&card.name, max_name_chars)
                },
                stats,
                footer,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Build panel: three stacked rows of items (next picks, full build, alternatives)
// ---------------------------------------------------------------------------

/// One item as the build panel shows it. `reason` is the engine's own
/// explanation, so the overlay never invents a justification the maths did not
/// produce.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OverlayBuildItemView {
    pub item_id: u32,
    pub name: String,
    pub win_rate: f64,
    pub lift: f64,
    pub sample_size: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OverlayBuildEntry {
    pub item_id: u32,
    pub rank: u8,
    pub name: String,
    /// "58.2% WR   +4.5%   1.2k"
    pub stats: String,
    /// "+4.5%" — for the two-column alternative row, where there is no width
    /// for a full stat line.
    pub compact: String,
    /// Why this item, in the engine's words.
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OverlayBuildPanel {
    /// "NEXT ITEMS · 52.4% est."
    pub headline: String,
    pub next_items: Vec<OverlayBuildEntry>,
    pub full_build: Vec<OverlayBuildEntry>,
    pub alternatives: Vec<OverlayBuildEntry>,
    /// "12.4k games indexed" / empty while the engine is still loading.
    pub footer: String,
}

impl OverlayBuildPanel {
    pub fn is_empty(&self) -> bool {
        self.next_items.is_empty() && self.full_build.is_empty() && self.alternatives.is_empty()
    }
}

fn item_stats(win_rate: f64, lift: f64, games: usize) -> String {
    format!(
        "{:.1}% WR   {:+.1}%   {}",
        win_rate * 100.0,
        lift * 100.0,
        compact_games(games)
    )
}

fn entry_from_view(view: &OverlayBuildItemView, rank: u8, max_name_chars: usize) -> OverlayBuildEntry {
    OverlayBuildEntry {
        item_id: view.item_id,
        rank,
        name: truncate_chars(&view.name, max_name_chars),
        stats: item_stats(view.win_rate, view.lift, view.sample_size),
        compact: format!("{:+.1}%", view.lift * 100.0),
        reason: truncate_chars(&view.reason, 44),
    }
}

/// Next picks carry a rank and the engine's reason; the full build only needs
/// order and name; alternatives are compact so they get the numbers, not prose.
pub fn build_build_panel(
    next_items: &[OverlayBuildItemView],
    full_build: &[OverlayBuildItemView],
    alternatives: &[OverlayBuildItemView],
    build_win_rate: f64,
    total_games: usize,
    max_name_chars: usize,
) -> OverlayBuildPanel {
    let headline = if build_win_rate > 0.0 {
        format!("NEXT ITEMS  ·  {:.1}% est.", build_win_rate * 100.0)
    } else {
        "NEXT ITEMS".to_string()
    };

    OverlayBuildPanel {
        headline,
        next_items: next_items
            .iter()
            .enumerate()
            .map(|(i, v)| entry_from_view(v, (i + 1) as u8, max_name_chars))
            .collect(),
        full_build: full_build
            .iter()
            .enumerate()
            .map(|(i, v)| entry_from_view(v, (i + 1) as u8, max_name_chars))
            .collect(),
        alternatives: alternatives
            .iter()
            .enumerate()
            .map(|(i, v)| entry_from_view(v, (i + 1) as u8, 18))
            .collect(),
        footer: if total_games > 0 {
            format!("{} games indexed", compact_games(total_games))
        } else {
            String::new()
        },
    }
}

pub fn build_hud_text(champion: &str, level: u32, gold: f64, has_cards: bool) -> String {
    let champion = if champion.trim().is_empty() {
        "ARAM MAYHEM"
    } else {
        champion.trim()
    };
    let mut text = champion.to_uppercase();
    if level > 0 {
        text.push_str(&format!(" • LV {level}"));
    }
    if gold > 0.0 {
        text.push_str(&format!(" • {:.0}g", gold));
    }
    text.push_str(if has_cards {
        " • F8 RESCAN"
    } else {
        " • F8 SCAN"
    });
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::history::{OverlayAugmentCard, OverlayComboEntry};

    fn card(slot: usize, name: &str, decision: &str, rank: u8) -> OverlayAugmentCard {
        OverlayAugmentCard {
            slot,
            id: name.to_string(),
            name: name.to_string(),
            rarity: Some("Gold".to_string()),
            wpa: 0.045,
            win_rate: 0.582,
            games: 1240,
            synergy_items: Vec::new(),
            combos: vec![OverlayComboEntry {
                label: "Nashor's Tooth".to_string(),
                win_rate: 0.61,
                wpa: 0.011,
                games: 800,
                items: Vec::new(),
            }],
            decision: Some(decision.to_string()),
            rank: Some(rank),
            ..Default::default()
        }
    }

    #[test]
    fn rows_follow_screen_order_not_score_order() {
        let cards = vec![card(2, "Third", "skip", 3), card(0, "First", "pick", 1)];
        let rows = build_rows(&cards, 26);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].slot, 0);
        assert_eq!(rows[1].slot, 2);
    }

    #[test]
    fn header_carries_rank_slot_and_verdict() {
        let rows = build_rows(&[card(0, "Phenomenal Evil", "pick", 1)], 26);
        assert_eq!(rows[0].header, "1. LEFT  •  PICK  [F9]");
        assert_eq!(rows[0].name, "Phenomenal Evil");
        assert_eq!(rows[0].stats, "58.2% WR   +4.5%   1.2k");
        assert_eq!(rows[0].footer, "SYN: Nashor's Tooth +1.1%");
        assert_eq!(rows[0].accent, ACCENT_PICK);
    }

    #[test]
    fn unconfirmed_cards_are_not_presented_as_fact() {
        // Shared art + an unreadable title: the name is a guess, so the row must
        // not show stats that look authoritative.
        let card = OverlayAugmentCard {
            slot: 0,
            id: "2073".to_string(),
            name: "Siphon".to_string(),
            unconfirmed: true,
            ..Default::default()
        };
        let rows = build_rows(&[card], 40);
        assert_eq!(rows[0].name, "Siphon ?");
        assert!(rows[0].stats.contains("unread"), "{}", rows[0].stats);
    }

    #[test]
    fn pending_cards_say_reading_stats_not_zero_percent() {
        let mut reading = card(0, "Jeweled Gauntlet", "", 0);
        reading.pending = true;
        reading.games = 0;
        reading.win_rate = 0.0;
        reading.wpa = 0.0;
        let rows = build_rows(&[reading], 26);
        assert_eq!(rows[0].stats, "reading stats…");

        // Computed, but the augment has no sample at all.
        let mut empty = card(1, "Brand New", "pick", 1);
        empty.pending = false;
        empty.games = 0;
        let rows = build_rows(&[empty], 26);
        assert_eq!(rows[0].stats, "no sample yet");
    }

    #[test]
    fn every_row_shows_its_pick_hotkey() {
        let cards = vec![card(0, "A", "pick", 1), card(1, "B", "pick", 2), card(2, "C", "skip", 3)];
        let rows = build_rows(&cards, 26);
        assert!(rows[0].header.ends_with("[F9]"));
        assert!(rows[1].header.ends_with("[F10]"));
        assert!(rows[2].header.ends_with("[F11]"));
    }

    #[test]
    fn verdicts_map_to_expected_accents() {
        let pick = build_rows(&[card(0, "A", "pick", 1)], 26);
        let reroll = build_rows(&[card(1, "B", "reroll", 1)], 26);
        let skip = build_rows(&[card(2, "C", "skip", 2)], 26);
        let wait = build_rows(&[card(0, "D", "", 0)], 26);
        assert_eq!(pick[0].accent, ACCENT_PICK);
        assert_eq!(reroll[0].accent, ACCENT_REROLL);
        assert_eq!(skip[0].accent, ACCENT_SKIP);
        assert_eq!(wait[0].accent, ACCENT_SKIP);
        assert!(wait[0].header.contains("WAIT"));
    }

    #[test]
    fn long_names_are_truncated_with_an_ellipsis() {
        let rows = build_rows(
            &[card(0, "Supercalifragilisticexpialidocious", "pick", 1)],
            10,
        );
        assert_eq!(rows[0].name.chars().count(), 10);
        assert!(rows[0].name.ends_with('…'));
    }

    #[test]
    fn games_are_compacted_above_one_thousand() {
        assert_eq!(compact_games(999), "999");
        assert_eq!(compact_games(1000), "1.0k");
        assert_eq!(compact_games(1240), "1.2k");
    }

    #[test]
    fn hud_text_shows_vitals_and_hotkeys() {
        assert_eq!(
            build_hud_text("Jinx", 12, 1350.4, true),
            "JINX • LV 12 • 1350g • F8 RESCAN"
        );
        assert_eq!(build_hud_text("", 0, 0.0, false), "ARAM MAYHEM • F8 SCAN");
    }

    fn view(id: u32, name: &str, lift: f64) -> OverlayBuildItemView {
        OverlayBuildItemView {
            item_id: id,
            name: name.to_string(),
            win_rate: 0.55 + lift,
            lift,
            sample_size: 1200,
            reason: "Pairs with Jeweled Gauntlet (+2.1%)".to_string(),
        }
    }

    #[test]
    fn build_panel_keeps_three_row_groups_in_engine_order() {
        let next = vec![view(6655, "Luden's Companion", 0.045), view(3089, "Rabadon's", 0.021)];
        let full = vec![view(3020, "Sorcerer's Shoes", 0.01), view(6653, "Liandry's", 0.001)];
        let alts = vec![view(3157, "Zhonya's", 0.004)];
        let panel = build_build_panel(&next, &full, &alts, 0.5412, 12400, 26);

        assert_eq!(panel.headline, "NEXT ITEMS  ·  54.1% est.");
        assert_eq!(panel.next_items.len(), 2);
        assert_eq!(panel.next_items[0].rank, 1);
        assert_eq!(panel.next_items[1].rank, 2);
        assert_eq!(panel.next_items[0].name, "Luden's Companion");
        assert_eq!(panel.next_items[0].stats, "59.5% WR   +4.5%   1.2k");
        assert_eq!(panel.next_items[0].reason, "Pairs with Jeweled Gauntlet (+2.1%)");
        assert_eq!(panel.full_build.len(), 2);
        assert_eq!(panel.alternatives.len(), 1);
        assert_eq!(panel.footer, "12.4k games indexed");
        assert!(!panel.is_empty());
    }

    #[test]
    fn build_panel_reasons_are_truncated_for_the_narrow_row() {
        let mut long = view(6655, "Luden's", 0.04);
        long.reason = "Synergy +3.1% with Jeweled Gauntlet and Rabadon's Deathcap, cheap spike".to_string();
        let panel = build_build_panel(&[long], &[], &[], 0.0, 0, 40);
        assert!(panel.next_items[0].reason.chars().count() <= 44);
        assert!(panel.next_items[0].reason.ends_with('…'));
        // No sample data yet: no fake headline, no fake footer.
        assert_eq!(panel.headline, "NEXT ITEMS");
        assert_eq!(panel.footer, "");
    }

    #[test]
    fn empty_recommendation_yields_an_empty_panel() {
        let panel = build_build_panel(&[], &[], &[], 0.0, 0, 26);
        assert!(panel.is_empty());
    }
}
