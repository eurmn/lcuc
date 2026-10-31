use std::path::Path;

#[tokio::test]
async fn test_crop_cards_76() {
    let example_dir = Path::new("../@llm/cache/augment-selection-examples");
    let file = example_dir.join("Screen76.png");
    let img_bytes = std::fs::read(&file).unwrap();
    let dyn_img = image::load_from_memory(&img_bytes).expect("decode png");
    let rgba = dyn_img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();

    // Look for lines of text in card columns:
    // Left card: x in 600..1000
    // Mid card: x in 1080..1480
    // Right card: x in 1560..1960
    // y in 350..1000
    for y in (350..1000).step_by(10) {
        let count_l: usize = (600..1000).filter(|&x| is_text_pixel(&rgba, x, y)).count();
        let count_m: usize = (1080..1480).filter(|&x| is_text_pixel(&rgba, x, y)).count();
        let count_r: usize = (1560..1960).filter(|&x| is_text_pixel(&rgba, x, y)).count();
        if count_l > 5 || count_m > 5 || count_r > 5 {
            println!("y={}: Left={}, Mid={}, Right={}", y, count_l, count_m, count_r);
        }
    }
}

fn is_text_pixel(img: &image::RgbaImage, x: u32, y: u32) -> bool {
    let p = img.get_pixel(x, y);
    // Gold/white/cyan text on dark card background
    let r = p[0] as f32;
    let g = p[1] as f32;
    let b = p[2] as f32;
    let brightness = (r + g + b) / 3.0;
    // High brightness and not pure background
    brightness > 160.0
}
