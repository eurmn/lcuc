use lcu_companion_lib::services::live_overlay::recognize_text_lines_public_for_test;
use std::path::Path;

#[tokio::test]
async fn test_screen74_variations() {
    let example_dir = Path::new("../@llm/cache/augment-selection-examples");
    let file = example_dir.join("Screen74.png");
    let img_bytes = std::fs::read(&file).unwrap();
    let dyn_img = image::load_from_memory(&img_bytes).expect("decode png");
    let rgba = dyn_img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();

    let crop_x = (0.15 * width as f64) as u32;
    let crop_y = (0.18 * height as f64) as u32;
    let crop_w = (0.70 * width as f64) as u32;
    let crop_h = (0.54 * height as f64) as u32;

    for threshold in [100.0, 115.0, 130.0, 145.0, 160.0] {
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

        let lines = recognize_text_lines_public_for_test(&bgra, crop_w, crop_h).await.unwrap();
        println!("Threshold {}: {} lines", threshold, lines.len());
        for (rx, text) in &lines {
            println!("  [rx={:.3}] {}", rx, text);
        }
    }

    // Also test raw cropped without binarization
    let mut raw_bgra = Vec::with_capacity((crop_w * crop_h * 4) as usize);
    for y in crop_y..(crop_y + crop_h) {
        for x in crop_x..(crop_x + crop_w) {
            let p = rgba.get_pixel(x, y);
            raw_bgra.push(p[2]);
            raw_bgra.push(p[1]);
            raw_bgra.push(p[0]);
            raw_bgra.push(p[3]);
        }
    }
    let lines_raw = recognize_text_lines_public_for_test(&raw_bgra, crop_w, crop_h).await.unwrap();
    println!("Raw crop: {} lines", lines_raw.len());
    for (rx, text) in &lines_raw {
        println!("  [rx={:.3}] {}", rx, text);
    }
}
