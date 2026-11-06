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

/// One row per offered card, in screen order (LEFT, MID, RIGHT).
pub fn build_rows(cards: &[OverlayAugmentCard], max_name_chars: usize) -> Vec<OverlayCardRow> {
    let mut ordered: Vec<&OverlayAugmentCard> = cards.iter().take(3).collect();
    ordered.sort_by_key(|card| card.slot);

    ordered
        .iter()
        .map(|card| {
            let (verdict, accent) = verdict_label_and_accent(card.decision.as_deref());
            let rank = card.rank.unwrap_or(0);
            let header = if rank > 0 {
                format!("{}. {}  •  {}", rank, slot_label(card.slot), verdict)
            } else {
                format!("{}  •  {}", slot_label(card.slot), verdict)
            };
            let stats = format!(
                "{:.1}% WR   {:+.1}%   {}",
                card.win_rate * 100.0,
                card.wpa * 100.0,
                compact_games(card.games)
            );
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
                name: truncate_chars(&card.name, max_name_chars),
                stats,
                footer,
            }
        })
        .collect()
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
        assert_eq!(rows[0].header, "1. LEFT  •  PICK");
        assert_eq!(rows[0].name, "Phenomenal Evil");
        assert_eq!(rows[0].stats, "58.2% WR   +4.5%   1.2k");
        assert_eq!(rows[0].footer, "SYN: Nashor's Tooth +1.1%");
        assert_eq!(rows[0].accent, ACCENT_PICK);
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
        assert!(wait[0].header.ends_with("WAIT"));
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
}
