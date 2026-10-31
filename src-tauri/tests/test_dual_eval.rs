use lcu_companion_lib::models::history::{MayhemAugmentItem, MayhemAugmentsQuery};
use lcu_companion_lib::services::history::get_mayhem_augments;
use lcu_companion_lib::services::live_overlay::recognize_text_lines_public_for_test;
use lcu_companion_lib::services::dual_matcher::{DualSignalMatcher, normalize_token};
use std::path::Path;

#[tokio::test]
async fn test_dual_signal_matcher_on_all_samples() {
    let example_dir = Path::new("../@llm/cache/augment-selection-examples");
    let entries = std::fs::read_dir(example_dir).expect("read_dir examples");
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("png") {
            files.push(path);
        }
    }
    files.sort();

    // Fetch augment catalog
    let catalog = get_mayhem_augments(MayhemAugmentsQuery {
        page_size: Some(1000),
        ..Default::default()
    }).await.expect("load augments");
    println!("Loaded {} catalog augments.", catalog.items.len());

    let matcher = DualSignalMatcher::default();

    for file in &files {
        println!("\n========================================================");
        println!("EVALUATING: {:?}", file.file_name().unwrap());
        let img_bytes = std::fs::read(file).unwrap();
        let dyn_img = image::load_from_memory(&img_bytes).expect("decode png");
        let rgba = dyn_img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();

        let crop_x = (0.15 * width as f64) as u32;
        let crop_y = (0.18 * height as f64) as u32;
        let crop_w = (0.70 * width as f64) as u32;
        let crop_h = (0.54 * height as f64) as u32;

        let mut lines = Vec::new();
        for threshold in [130.0, 150.0] {
            let mut bgra = Vec::with_capacity((crop_w * crop_h * 4) as usize);
            for y in crop_y..(crop_y + crop_h) {
                for x in crop_x..(crop_x + crop_w) {
                    let p = rgba.get_pixel(x, y);
                    let lum = 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
                    if lum > threshold {
                        bgra.push(255);
                        bgra.push(255);
                        bgra.push(255);
                        bgra.push(255);
                    } else {
                        bgra.push(0);
                        bgra.push(0);
                        bgra.push(0);
                        bgra.push(255);
                    }
                }
            }
            if let Ok(l) = recognize_text_lines_public_for_test(&bgra, crop_w, crop_h).await {
                if !l.is_empty() {
                    lines = l;
                    break;
                }
            }
        }

        // Group lines by 3 card columns
        let mut cols: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for (rx, text) in &lines {
            let trimmed = text.trim();
            if trimmed.is_empty() { continue; }
            let col = if *rx < 0.38 { 0 } else if *rx < 0.65 { 1 } else { 2 };
            cols[col].push(trimmed.to_string());
        }

        for (c_idx, col_lines) in cols.iter().enumerate() {
            let col_joined = col_lines.join(" ");
            let col_text = normalize_token(&col_joined);
            let col_words: Vec<String> = col_joined
                .split_whitespace()
                .map(normalize_token)
                .filter(|w| !w.is_empty())
                .collect();

            let mut best_aug = None;
            let mut best_score = 0.0;
            let mut best_details = (0.0, 0.0);

            for aug in &catalog.items {
                let (tot, t_sc, d_sc) = matcher.score_candidate(col_lines, &col_text, &col_words, aug);
                if tot > best_score && tot >= 0.55 {
                    best_score = tot;
                    best_aug = Some(aug.clone());
                    best_details = (t_sc, d_sc);
                }
            }

            if let Some(aug) = best_aug {
                println!("  [Card {}] MATCH: '{}' (Score: {:.3}, Title: {:.2}, Desc: {:.2}) lines={:?}", 
                    c_idx + 1, aug.name, best_score, best_details.0, best_details.1, col_lines);
            } else {
                println!("  [Card {}] NO MATCH (Lines: {:?})", c_idx + 1, col_lines);
            }
        }
    }
}
