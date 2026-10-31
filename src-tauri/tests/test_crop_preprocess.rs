use lcu_companion_lib::services::live_overlay::recognize_text_lines_public_for_test;
use std::path::Path;

#[tokio::test]
async fn test_crop_preprocess() {
    let example_dir = Path::new("../@llm/cache/augment-selection-examples");
    let file = example_dir.join("Screen76.png");
    let img_bytes = std::fs::read(&file).unwrap();
    let dyn_img = image::load_from_memory(&img_bytes).expect("decode png");
    let rgba = dyn_img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();

    let crop_x = (0.20 * width as f64) as u32;
    let crop_y = (0.22 * height as f64) as u32;
    let crop_w = (0.60 * width as f64) as u32;
    let crop_h = (0.45 * height as f64) as u32;

    // Test 1: Original crop
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

    // Test 2: Inverted or High Contrast
    // Boost text brightness, darken background
    let mut high_contrast = bgra.clone();
    for p in high_contrast.chunks_exact_mut(4) {
        let b = p[0] as f32;
        let g = p[1] as f32;
        let r = p[2] as f32;
        let lum = 0.299 * r + 0.587 * g + 0.114 * b;
        if lum > 140.0 {
            // White text
            p[0] = 255;
            p[1] = 255;
            p[2] = 255;
        } else {
            // Black bg
            p[0] = 0;
            p[1] = 0;
            p[2] = 0;
        }
    }

    let lines = recognize_text_lines_public_for_test(&high_contrast, crop_w, crop_h).await.unwrap();
    println!("High contrast crop 76 recognized {} lines:", lines.len());
    for (rx, text) in lines {
        println!("  [rx={:.3}] {}", rx, text);
    }

    // Test 3: Downscaled
    let scale = 0.5;
    let dw = (crop_w as f32 * scale) as u32;
    let dh = (crop_h as f32 * scale) as u32;
    let mut downscaled = vec![0u8; (dw * dh * 4) as usize];
    for dy in 0..dh {
        for dx in 0..dw {
            let sx = (dx as f32 / scale) as u32;
            let sy = (dy as f32 / scale) as u32;
            let sp = ((sy * crop_w + sx) * 4) as usize;
            let dp = ((dy * dw + dx) * 4) as usize;
            downscaled[dp..dp+4].copy_from_slice(&bgra[sp..sp+4]);
        }
    }
    let lines_down = recognize_text_lines_public_for_test(&downscaled, dw, dh).await.unwrap();
    println!("Downscaled (0.5) crop 76 recognized {} lines:", lines_down.len());
    for (rx, text) in lines_down {
        println!("  [rx={:.3}] {}", rx, text);
    }
}
