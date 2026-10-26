use crate::logger;
use crate::models::{MayhemAugmentsQuery, MayhemAugmentsResponse, MayhemChampionBuildSummary};
use crate::services::history::{compute_mayhem_champion_build, get_mayhem_augments};
use crate::services::lcu_client::LcuClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ExportBuildPayload {
    pub champion_id: i64,
    pub champion_name: String,
    pub strategy_name: String,
    #[serde(default)]
    pub starter_item_id: Option<i64>,
    pub item_ids: Vec<i64>,
    pub spell1_id: u64,
    pub spell2_id: u64,
}

#[derive(Debug, Serialize)]
pub struct ExportBuildResult {
    pub success: bool,
    pub item_set_created: bool,
    pub spells_applied: bool,
    pub message: String,
}

/// Queries the active Champion Select session from LCU.
/// Returns `Ok(Some(session))` if currently in Champ Select, or `Ok(None)` if idle or not in Champ Select.
#[tauri::command]
pub async fn api_champ_select_session() -> Result<Option<Value>, String> {
    let Some(creds) = LcuClient::find_credentials() else {
        return Ok(None);
    };
    let Ok(client) = LcuClient::new(creds) else {
        return Ok(None);
    };

    match client
        .get_json_response::<Value>("/lol-champ-select/v1/session")
        .await
    {
        Ok(session) => Ok(Some(session)),
        Err(err) => {
            if err.status == Some(404) {
                Ok(None)
            } else {
                logger::debug(
                    "tauri.champ_select",
                    &format!("Champ select session check: {}", err.message),
                );
                Ok(None)
            }
        }
    }
}

/// Computes or retrieves from cache the full 8-strategy build analysis for a champion in ARAM Mayhem.
#[tauri::command]
pub async fn api_mayhem_champion_build(
    champion_id: i64,
) -> Result<MayhemChampionBuildSummary, String> {
    compute_mayhem_champion_build(champion_id).await
}

/// Batch-computes or retrieves builds for multiple champions (team roster and bench).
#[tauri::command]
pub async fn api_mayhem_team_builds(
    champion_ids: Vec<i64>,
) -> Result<HashMap<i64, MayhemChampionBuildSummary>, String> {
    let mut results = HashMap::new();
    for id in champion_ids {
        if id <= 0 {
            continue;
        }
        if let Ok(summary) = compute_mayhem_champion_build(id).await {
            results.insert(id, summary);
        }
    }
    Ok(results)
}

/// Queries the complete augments list with server-side pagination, tier & category filtering, and Bayesian WPA ranking.
#[tauri::command]
pub async fn api_mayhem_augments(
    query: MayhemAugmentsQuery,
) -> Result<MayhemAugmentsResponse, String> {
    get_mayhem_augments(query).await
}

/// Exports the recommended items to a League Client item set and applies summoner spells to the active champ select.
#[tauri::command]
pub async fn api_export_mayhem_build(
    payload: ExportBuildPayload,
) -> Result<ExportBuildResult, String> {
    let creds = LcuClient::find_credentials()
        .ok_or_else(|| "League Client is not running or credentials not found".to_string())?;
    let client = LcuClient::new(creds).map_err(|e| format!("Failed creating LCU client: {e}"))?;

    // 1. Get current summoner ID
    let summoner: Value = client
        .get_json("/lol-summoner/v1/current-summoner")
        .await
        .map_err(|e| format!("Failed getting current summoner: {e}"))?;
    let summoner_id = summoner
        .get("summonerId")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Could not determine summonerId".to_string())?;

    // 2. Fetch existing item sets
    let item_sets_path = format!("/lol-item-sets/v1/item-sets/{summoner_id}/sets");
    let mut sets_wrapper: Value = client
        .get_json(&item_sets_path)
        .await
        .unwrap_or_else(|_| json!({ "itemSets": [] }));

    let mut item_set_created = false;
    let target_title = format!(
        "[LCUC] {} - {}",
        payload.champion_name, payload.strategy_name
    );
    let champ_prefix = format!("[LCUC] {}", payload.champion_name);

    let mut starter_items: Vec<Value> = Vec::new();
    if let Some(starter_id) = payload.starter_item_id {
        if starter_id > 0 {
            starter_items.push(json!({ "id": starter_id.to_string(), "count": 1 }));
            starter_items.push(json!({ "id": "2003", "count": 1 }));
        }
    }
    if starter_items.is_empty() {
        starter_items.push(json!({ "id": "2003", "count": 1 }));
    }

    let core_items: Vec<Value> = payload
        .item_ids
        .iter()
        .map(|id| json!({ "id": id.to_string(), "count": 1 }))
        .collect();

    let new_set = json!({
        "title": target_title,
        "associatedChampions": [payload.champion_id],
        "associatedMaps": [12],
        "blocks": [
            {
                "type": "Starting Items",
                "items": starter_items,
                "hideIfSummonerSpell": "",
                "showIfSummonerSpell": ""
            },
            {
                "type": "Core & Recommended Build",
                "items": core_items,
                "hideIfSummonerSpell": "",
                "showIfSummonerSpell": ""
            }
        ],
        "map": "any",
        "mode": "any",
        "preferredItemSlots": [],
        "sortrank": 0,
        "startedFrom": "blank",
        "type": "custom",
        "uid": format!("lcuc-{}-{}", payload.champion_id, payload.strategy_name.to_lowercase().replace(' ', "-"))
    });

    if let Some(sets) = sets_wrapper
        .get_mut("itemSets")
        .and_then(Value::as_array_mut)
    {
        let existing_index = sets.iter().position(|s| {
            s.get("title")
                .and_then(Value::as_str)
                .map(|t| t == target_title || t.starts_with(&champ_prefix))
                .unwrap_or(false)
        });

        if let Some(idx) = existing_index {
            sets[idx] = new_set;
        } else {
            sets.push(new_set);
        }

        let put_res = client.put_json_status(&item_sets_path, &sets_wrapper).await;
        if put_res.is_ok() {
            item_set_created = true;
        }
    }

    // 3. Apply summoner spells if in active Champ Select
    let mut spells_applied = false;
    if payload.spell1_id > 0 && payload.spell2_id > 0 {
        let spells_body = json!({
            "spell1Id": payload.spell1_id,
            "spell2Id": payload.spell2_id
        });
        if let Ok(status) = client
            .patch_json_status("/lol-champ-select/v1/session/my-selection", &spells_body)
            .await
        {
            if status >= 200 && status < 300 {
                spells_applied = true;
            }
        }
    }

    let message = if item_set_created && spells_applied {
        format!(
            "Item set \"{}\" and Summoner Spells saved & applied!",
            target_title
        )
    } else if item_set_created {
        format!("Item set \"{}\" created in client inventory!", target_title)
    } else if spells_applied {
        "Summoner spells applied to champion selection!".to_string()
    } else {
        "Failed exporting to League Client".to_string()
    };

    Ok(ExportBuildResult {
        success: item_set_created || spells_applied,
        item_set_created,
        spells_applied,
        message,
    })
}
