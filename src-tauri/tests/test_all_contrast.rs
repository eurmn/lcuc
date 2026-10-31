use lcu_companion_lib::services::live_overlay::recognize_text_lines_public_for_test;
use std::path::Path;

#[tokio::test]
async fn test_all_images_with_contrast() {
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

    for file in &files {
        println!("\n========================================================");
        println!("TESTING FILE: {:?}", file.file_name().unwrap());
        let img_bytes = std::fs::read(file).unwrap();
        let dyn_img = image::load_from_memory(&img_bytes).expect("decode png");
        let rgba = dyn_img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();

        // Cards region: x: 0.15..0.85, y: 0.18..0.72
        let crop_x = (0.15 * width as f64) as u32;
        let crop_y = (0.18 * height as f64) as u32;
        let crop_w = (0.70 * width as f64) as u32;
        let crop_h = (0.54 * height as f64) as u32;

        let mut bgra = Vec::with_capacity((crop_w * crop_h * 4) as usize);
        for y in crop_y..(crop_y + crop_h) {
            for x in crop_x..(crop_x + crop_w) {
                let p = rgba.get_pixel(x, y);
                // Binarization / High Contrast:
                // Text in League card art is high luminance (white, gold, cyan)
                let r = p[0] as f32;
                let g = p[1] as f32;
                let b = p[2] as f32;
                let lum = 0.299 * r + 0.587 * g + 0.114 * b;
                if lum > 130.0 {
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
        println!("Recognized {} lines:", lines.len());
        for (rx, text) in lines {
            println!("  [rx={:.3}] {}", rx, text);
        }
    }
}
