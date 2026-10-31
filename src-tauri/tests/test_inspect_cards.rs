use std::path::Path;

#[tokio::test]
async fn test_inspect_card_area() {
    let example_dir = Path::new("../@llm/cache/augment-selection-examples");
    let files = vec![
        example_dir.join("Screen68.png"),
        example_dir.join("Screen75.png"),
        example_dir.join("Screen76.png"),
    ];

    for file in &files {
        println!("\n========================================================");
        println!("INVESTIGATING: {:?}", file.file_name().unwrap());
        let img_bytes = std::fs::read(file).unwrap();
        let dyn_img = image::load_from_memory(&img_bytes).expect("decode png");
        let rgba = dyn_img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();

        // Let's find bounding boxes of bright text or pixels
        for cy_ratio in [0.25, 0.30, 0.35, 0.40, 0.45, 0.50, 0.55, 0.60] {
            let y = (cy_ratio * height as f64) as u32;
            let mut bright_count = 0;
            for x in 0..width {
                let p = rgba.get_pixel(x, y);
                // brightness
                let b = (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3;
                if b > 180 {
                    bright_count += 1;
                }
            }
            println!("y={:4} (ratio {:.2}): bright pixels = {}", y, cy_ratio, bright_count);
        }
    }
}
