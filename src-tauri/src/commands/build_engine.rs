#![allow(non_snake_case)]

use crate::agent::mayhem_engine::{
    self, compute_mayhem_recommendation, query_mayhem_combos as engine_query_combos,
    MayhemBuildRecommendationRequest, MayhemBuildRecommendationResponse, MayhemComboDto,
    MAYHEM_SYNERGY_INDEX,
};
use crate::agent::{database_path, global_aram_database_path};

pub(crate) async fn ensure_index_ready() -> Result<(), String> {
    let needs_build = {
        let read = MAYHEM_SYNERGY_INDEX.read().map_err(|e| e.to_string())?;
        read.is_none()
    };
    if needs_build {
        tokio::task::spawn_blocking(|| {
            let global_path = global_aram_database_path();
            let agent_path = database_path();
            let index = mayhem_engine::build_synergy_index(&global_path, &agent_path)?;
            let mut write = MAYHEM_SYNERGY_INDEX.write().map_err(|e| e.to_string())?;
            *write = Some(index);
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| e.to_string())??;
    }
    Ok(())
}

#[tauri::command]
pub async fn compute_mayhem_build_recommendation(
    champion_id: Option<u32>,
    championId: Option<u32>,
    locked_augments: Option<Vec<String>>,
    lockedAugments: Option<Vec<String>>,
    owned_augments: Option<Vec<String>>,
    ownedAugments: Option<Vec<String>>,
    locked_items: Option<Vec<u32>>,
    lockedItems: Option<Vec<u32>>,
    owned_items: Option<Vec<u32>>,
    ownedItems: Option<Vec<u32>>,
    offered_cards: Option<Vec<String>>,
    offeredCards: Option<Vec<String>>,
    seen_augments: Option<Vec<String>>,
    seenAugments: Option<Vec<String>>,
    augment_slot: Option<u8>,
    augmentSlot: Option<u8>,
    command_line_reroll: Option<bool>,
    commandLineReroll: Option<bool>,
) -> Result<MayhemBuildRecommendationResponse, String> {
    ensure_index_ready().await?;

    let champ_id = champion_id.or(championId).unwrap_or(0);
    let augs = locked_augments
        .or(lockedAugments)
        .or(owned_augments)
        .or(ownedAugments)
        .unwrap_or_default();
    let items = locked_items
        .or(lockedItems)
        .or(owned_items)
        .or(ownedItems)
        .unwrap_or_default();
    let offered = offered_cards.or(offeredCards);
    let seen = seen_augments.or(seenAugments);
    let slot = augment_slot.or(augmentSlot);
    let reroll = command_line_reroll.or(commandLineReroll);

    let req = MayhemBuildRecommendationRequest {
        champion_id: champ_id,
        locked_augments: augs,
        locked_items: items,
        offered_cards: offered,
        seen_augments: seen,
        augment_slot: slot,
        command_line_reroll: reroll,
    };

    tokio::task::spawn_blocking(move || {
        let read = MAYHEM_SYNERGY_INDEX.read().map_err(|e| e.to_string())?;
        let Some(ref index) = *read else {
            return Err("Synergy index unavailable".to_string());
        };
        Ok(compute_mayhem_recommendation(req, index))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn api_compute_mayhem_build_recommendation(
    champion_id: Option<u32>,
    championId: Option<u32>,
    locked_augments: Option<Vec<String>>,
    lockedAugments: Option<Vec<String>>,
    owned_augments: Option<Vec<String>>,
    ownedAugments: Option<Vec<String>>,
    locked_items: Option<Vec<u32>>,
    lockedItems: Option<Vec<u32>>,
    owned_items: Option<Vec<u32>>,
    ownedItems: Option<Vec<u32>>,
    offered_cards: Option<Vec<String>>,
    offeredCards: Option<Vec<String>>,
    seen_augments: Option<Vec<String>>,
    seenAugments: Option<Vec<String>>,
    augment_slot: Option<u8>,
    augmentSlot: Option<u8>,
    command_line_reroll: Option<bool>,
    commandLineReroll: Option<bool>,
) -> Result<MayhemBuildRecommendationResponse, String> {
    compute_mayhem_build_recommendation(
        champion_id,
        championId,
        locked_augments,
        lockedAugments,
        owned_augments,
        ownedAugments,
        locked_items,
        lockedItems,
        owned_items,
        ownedItems,
        offered_cards,
        offeredCards,
        seen_augments,
        seenAugments,
        augment_slot,
        augmentSlot,
        command_line_reroll,
        commandLineReroll,
    )
    .await
}

#[tauri::command]
pub async fn query_mayhem_combos(
    champion_id: Option<u16>,
    championId: Option<u16>,
    combo_category: Option<String>,
    comboCategory: Option<String>,
    sort_by: Option<String>,
    sortBy: Option<String>,
    descending: Option<bool>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<MayhemComboDto>, String> {
    ensure_index_ready().await?;

    let champ = champion_id.or(championId);
    let cat = combo_category.or(comboCategory);
    let sort = sort_by.or(sortBy);
    let desc = descending.unwrap_or(true);
    let lim = limit.unwrap_or(50);
    let off = offset.unwrap_or(0);

    tokio::task::spawn_blocking(move || {
        let read = MAYHEM_SYNERGY_INDEX.read().map_err(|e| e.to_string())?;
        let Some(ref index) = *read else {
            return Err("Synergy index unavailable".to_string());
        };
        Ok(engine_query_combos(
            index,
            champ,
            cat.as_deref(),
            sort.as_deref(),
            desc,
            lim,
            off,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn api_query_mayhem_combos(
    champion_id: Option<u16>,
    championId: Option<u16>,
    combo_category: Option<String>,
    comboCategory: Option<String>,
    sort_by: Option<String>,
    sortBy: Option<String>,
    descending: Option<bool>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<MayhemComboDto>, String> {
    query_mayhem_combos(
        champion_id,
        championId,
        combo_category,
        comboCategory,
        sort_by,
        sortBy,
        descending,
        limit,
        offset,
    )
    .await
}
