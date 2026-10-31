use lcu_companion_lib::services::live_overlay::scan_title_band_from_image;
use std::path::Path;

#[tokio::test]
async fn test_sample_images_ocr() {
    let example_dir = Path::new("../@llm/cache/augment-selection-examples");
    assert!(example_dir.exists(), "Examples directory must exist");

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

    println!("Found {} sample images to test:", files.len());
    for file in &files {
        println!("\n========================================================");
        println!("TESTING IMAGE: {:?}", file.file_name().unwrap());
        let img_bytes = std::fs::read(file).unwrap();
        let dyn_img = image::load_from_memory(&img_bytes).expect("decode png");
        let rgba = dyn_img.to_rgba8();

        let cards = scan_title_band_from_image(&rgba, None)
            .await
            .unwrap_or_else(|e| {
                println!("Error scanning {:?}: {}", file.file_name().unwrap(), e);
                Vec::new()
            });

        println!("Image {:?} recognized {} cards:", file.file_name().unwrap(), cards.len());
        for (i, card) in cards.iter().enumerate() {
            println!("  [Card {}] ID={}, Name={}", i, card.id, card.name);
        }
    }
}
