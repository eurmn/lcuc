use lcu_companion_lib::models::history::MayhemAugmentsQuery;
use lcu_companion_lib::services::history::get_mayhem_augments;

#[tokio::test]
async fn test_inspect_catalog_names() {
    let catalog = get_mayhem_augments(MayhemAugmentsQuery {
        page_size: Some(1000),
        ..Default::default()
    }).await.unwrap();

    let targets = [
        "Skilled Sniper", "Tooth Fairy", "From Downtown",
        "Pinball", "Thread the Needle", "Divine Intervention",
        "Flashy", "Upgrade Sheen", "Double Time",
        "Master of Duality",
        "Ok Boomerang", "Boomerang",
        "Final Form", "Critical Healing", "Recursion", "Infinite Recursion",
        "Brass Blast", "All For You",
        "Symphony of War", "Clown College", "Mystic Punch",
    ];

    println!("Total items: {}", catalog.items.len());
    for t in &targets {
        let found = catalog.items.iter().find(|a| a.name.to_lowercase() == t.to_lowercase());
        if let Some(a) = found {
            println!("FOUND: '{}' (id={}, tooltip_len={})", a.name, a.id, a.tooltip.as_ref().map(|s| s.len()).unwrap_or(0));
        } else {
            println!("MISSING: '{}'", t);
        }
    }
}
