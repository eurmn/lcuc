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
        use lcu_companion_lib::services::overlay_presenter::{
            build_build_panel, OverlayBuildItemView,
        };

        let view = |id: u32, name: &str, wr: f64, lift: f64, games: usize, reason: &str| {
            OverlayBuildItemView {
                item_id: id,
                name: name.to_string(),
                win_rate: wr,
                lift,
                sample_size: games,
                reason: reason.to_string(),
            }
        };

        let next = vec![
            view(6655, "Luden's Companion", 0.5612, 0.0421, 1840, "Pairs with Jeweled Gauntlet (+2.4% synergy)"),
            view(3089, "Rabadon's Deathcap", 0.5543, 0.0312, 2210, "Standalone: 3.1% win-rate lift over 2210 games"),
            view(6653, "Liandry's Torment", 0.5471, 0.0203, 980, "Pairs with Blistering Burn (+1.8% synergy)"),
        ];
        let full = vec![
            view(3020, "Sorcerer's Shoes", 0.55, 0.012, 4200, ""),
            view(6655, "Luden's Companion", 0.5612, 0.0421, 1840, ""),
            view(3089, "Rabadon's Deathcap", 0.5543, 0.0312, 2210, ""),
            view(6653, "Liandry's Torment", 0.5471, 0.0203, 980, ""),
            view(3157, "Zhonya's Hourglass", 0.5503, 0.0111, 1500, ""),
            view(4645, "Shadowflame", 0.5499, 0.0222, 760, ""),
        ];
        let alts = vec![
            view(3116, "Rylai's Crystal Scepter", 0.5461, 0.0113, 640, ""),
            view(3165, "Morellonomicon", 0.5442, 0.0091, 540, ""),
            view(3135, "Void Staff", 0.5438, 0.0087, 1200, ""),
            view(4637, "Horizon Focus", 0.5421, 0.0078, 430, ""),
        ];
        let panel = build_build_panel(&next, &full, &alts, 0.5412, 12400, 26);

        let payload = OverlayRenderPayload {
            visible: true,
            champion_name: "Brand".to_string(),
            hud_text: build_hud_text("Brand", 12, 1350.0, true),
            rows: build_rows(&cards, 26),
            build_panel: panel,
            ..Default::default()
        };

        render_overlay_png(&payload, 1920, 1080, "overlay-preview.png").expect("render preview");
        println!("wrote overlay-preview.png");
    }
}
