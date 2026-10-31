use lcu_companion_lib::services::live_overlay::recognize_text_lines_public_for_test;
use std::path::Path;

#[tokio::test]
async fn test_all_images_multi_pass() {
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
        println!("MULTI-PASS FOR: {:?}", file.file_name().unwrap());
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

        println!("Image {:?} got {} lines:", file.file_name().unwrap(), lines.len());
        for (rx, text) in &lines {
            println!("  [rx={:.3}] {}", rx, text);
        }
    }
}
