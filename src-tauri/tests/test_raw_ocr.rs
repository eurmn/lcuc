use lcu_companion_lib::services::live_overlay::recognize_text_lines_public_for_test;
use std::path::Path;

#[tokio::test]
async fn test_inspect_raw_ocr_lines() {
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
        println!("RAW OCR FOR: {:?}", file.file_name().unwrap());
        let img_bytes = std::fs::read(file).unwrap();
        let dyn_img = image::load_from_memory(&img_bytes).expect("decode png");
        let rgba = dyn_img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();
        println!("Image dims: {}x{}", width, height);

        let mut bgra = Vec::with_capacity((width * height * 4) as usize);
        for p in rgba.pixels() {
            bgra.push(p[2]);
            bgra.push(p[1]);
            bgra.push(p[0]);
            bgra.push(p[3]);
        }

        let lines = recognize_text_lines_public_for_test(&bgra, width, height)
            .await
            .unwrap();
        println!("Recognized {} raw lines:", lines.len());
        for (rel_x, text) in lines {
            println!("  [rel_x={:.3}] {}", rel_x, text);
        }
    }
}
