#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    {
        use lcu_companion_lib::models::history::{OverlayAugmentCard, OverlayComboEntry};
        use lcu_companion_lib::services::native_overlay::{
            render_overlay_png, OverlayRenderPayload,
        };
        use lcu_companion_lib::services::overlay_presenter::{build_hud_text, build_rows};

        let sample = |slot: usize, name: &str, decision: &str, rank: u8, wr: f64, wpa: f64| {
            OverlayAugmentCard {
                slot,
                id: name.to_string(),
                name: name.to_string(),
                rarity: Some("Gold".to_string()),
                wpa,
                win_rate: wr,
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
        };

        let cards = vec![
            sample(0, "Phenomenal Evil", "pick", 1, 0.582, 0.045),
            sample(1, "Marksmage", "skip", 2, 0.548, 0.012),
            sample(2, "Ethereal Weapon", "skip", 3, 0.521, -0.008),
        ];
        let payload = OverlayRenderPayload {
            visible: true,
            champion_name: "Brand".to_string(),
            hud_text: build_hud_text("Brand", 12, 1350.0, true),
            rows: build_rows(&cards, 26),
            ..Default::default()
        };

        render_overlay_png(&payload, 1920, 1080, "overlay-preview.png").expect("render preview");
        println!("wrote overlay-preview.png");
    }
}
