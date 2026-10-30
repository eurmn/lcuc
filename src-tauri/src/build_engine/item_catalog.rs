use serde::Deserialize;
use std::collections::HashSet;
use std::sync::LazyLock;

#[derive(Debug, Deserialize)]
struct CatalogData {
    legendary_ids: Vec<u32>,
    boot_ids: Vec<u32>,
    component_ids: Vec<u32>,
}

static CATALOG: LazyLock<CatalogData> = LazyLock::new(|| {
    let raw = include_str!("../data/legendary_items.json");
    serde_json::from_str(raw).unwrap_or_else(|e| {
        eprintln!("Failed to parse embedded legendary_items.json: {}", e);
        CatalogData {
            legendary_ids: Vec::new(),
            boot_ids: Vec::new(),
            component_ids: Vec::new(),
        }
    })
});

static LEGENDARY_SET: LazyLock<HashSet<u32>> =
    LazyLock::new(|| CATALOG.legendary_ids.iter().copied().collect());

static BOOT_SET: LazyLock<HashSet<u32>> =
    LazyLock::new(|| CATALOG.boot_ids.iter().copied().collect());

static COMPONENT_SET: LazyLock<HashSet<u32>> =
    LazyLock::new(|| CATALOG.component_ids.iter().copied().collect());

/// Returns true if the given item ID is a completed Legendary / Mythic item.
#[inline]
pub fn is_legendary_item(item_id: u32) -> bool {
    LEGENDARY_SET.contains(&item_id)
}

/// Returns true if the given item ID is a completed Tier-2+ boot.
#[inline]
pub fn is_completed_boot(item_id: u32) -> bool {
    BOOT_SET.contains(&item_id)
}

/// Returns true if the given item ID is a basic component item.
#[inline]
pub fn is_component_item(item_id: u32) -> bool {
    COMPONENT_SET.contains(&item_id)
}

/// Returns true if the item is acceptable in a final completed build (Legendary or Completed Boot).
#[inline]
pub fn is_valid_build_item(item_id: u32) -> bool {
    is_legendary_item(item_id) || is_completed_boot(item_id)
}

/// Returns true if an item is an exclusive augment reward (e.g. Wooglet's Witchcap, Golden Spatula, Ultra Hydra, Void Immolation, Jarvan I's).
#[inline]
pub fn is_augment_reward_item(item_id: u32) -> bool {
    matches!(
        item_id,
        228002          // Wooglet's Witchcap
        | 994403 | 4403 | 224403 | 664403 // Golden Spatula
        | 226668        // Ultra Hydra
        | 223069        // Void Immolation
        | 1111          // Jarvan I's
    )
}

/// Returns true if the player possesses the required augment for the specified augment-reward item.
pub fn player_has_augment_for_item(item_id: u32, owned_augments: &[String]) -> bool {
    let lower_augs: Vec<String> = owned_augments.iter().map(|a| a.to_lowercase()).collect();
    let has_any = |identifiers: &[&str]| {
        lower_augs.iter().any(|owned| {
            identifiers.iter().any(|&id| owned == id || owned.contains(id))
        })
    };

    match item_id {
        // Wooglet's Witchcap (Augment ID 1156 / 156 / "Quest: Wooglet's Witchcap")
        228002 => has_any(&["1156", "156", "wooglet"]),
        // Golden Spatula (Augment ID 1154 / 154 / "Quest: Urf's Champion")
        994403 | 4403 | 224403 | 664403 => has_any(&["1154", "154", "urf"]),
        // Ultra Hydra (Augment ID 2141 / "Quest: Ultra Hydra")
        226668 => has_any(&["2141", "ultrahydra", "ultra hydra"]),
        // Void Immolation (Augment ID 393 / 1361 / "Icathia's Fall" / "Void Immolation")
        223069 => has_any(&["393", "1361", "icathia", "voidimmolation", "void immolation"]),
        // Jarvan I's
        1111 => has_any(&["jarvan"]),
        // Any other item is standard / not an augment quest item
        _ => true,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_sets() {
        // Legendary items: Rabadon (3089), Infinity Edge (3031), Dusk and Dawn (2510)
        assert!(is_legendary_item(3089));
        assert!(is_legendary_item(3031));
        assert!(is_legendary_item(2510));

        // Boots: Sorcerer's Shoes (3020), Plated Steelcaps (3047)
        assert!(is_completed_boot(3020));
        assert!(is_completed_boot(3047));

        // Augment quest items
        assert!(is_augment_reward_item(228002)); // Wooglet
        assert!(is_augment_reward_item(994403)); // Golden Spatula
        assert!(is_augment_reward_item(226668)); // Ultra Hydra
        assert!(is_augment_reward_item(223069)); // Void Immolation
        assert!(!is_augment_reward_item(3089));  // Rabadon is not an augment reward

        // Player has augment check
        assert!(!player_has_augment_for_item(228002, &[]));
        assert!(player_has_augment_for_item(228002, &["1156".to_string()]));
        assert!(player_has_augment_for_item(228002, &["Quest: Wooglet's Witchcap".to_string()]));
        assert!(!player_has_augment_for_item(226668, &["1156".to_string()]));
        assert!(player_has_augment_for_item(226668, &["2141".to_string()]));
        assert!(player_has_augment_for_item(3089, &[])); // standard item always true
        assert!(!is_legendary_item(3020));

        // Components: Needlessly Large Rod (1058), B.F. Sword (1038), Boots of Speed (1001)
        assert!(is_component_item(1058));
        assert!(is_component_item(1038));
        assert!(!is_legendary_item(1058));
        assert!(!is_completed_boot(1058));

        // Valid build items: legendaries or boots
        assert!(is_valid_build_item(3089));
        assert!(is_valid_build_item(3020));
        assert!(!is_valid_build_item(1058));
    }
}
