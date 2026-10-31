use lcu_companion_lib::services::live_overlay::recognize_text_lines_public_for_test;
use std::path::Path;

#[tokio::test]
async fn test_crop_ocr_card_band() {
    let example_dir = Path::new("../@llm/cache/augment-selection-examples");
    let file = example_dir.join("Screen76.png");
    let img_bytes = std::fs::read(&file).unwrap();
    let dyn_img = image::load_from_memory(&img_bytes).expect("decode png");
    let rgba = dyn_img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();

    // Crop just the cards: x: 0.15..0.85, y: 0.20..0.70
    let crop_x = (0.15 * width as f64) as u32;
    let crop_y = (0.20 * height as f64) as u32;
    let crop_w = (0.70 * width as f64) as u32;
    let crop_h = (0.50 * height as f64) as u32;

    let mut bgra = Vec::with_capacity((crop_w * crop_h * 4) as usize);
    for y in crop_y..(crop_y + crop_h) {
        for x in crop_x..(crop_x + crop_w) {
            let p = rgba.get_pixel(x, y);
            bgra.push(p[2]);
            bgra.push(p[1]);
            bgra.push(p[0]);
            bgra.push(p[3]);
        }
    }

    let lines = recognize_text_lines_public_for_test(&bgra, crop_w, crop_h).await.unwrap();
    println!("Crop 76 recognized {} lines:", lines.len());
    for (rx, text) in lines {
        println!("  [rx={:.3}] {}", rx, text);
    }
}
