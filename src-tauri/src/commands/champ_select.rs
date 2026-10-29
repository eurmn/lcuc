use crate::logger;
use crate::models::{
    MayhemAugmentsQuery, MayhemAugmentsResponse, MayhemChampionBuildSummary, MayhemLiveAdvice,
    SaveMayhemBuildPayload, SavedMayhemBuild,
};
use crate::services::history::{
    compute_live_build_advice, compute_mayhem_champion_build, delete_saved_mayhem_build,
    get_mayhem_augments, list_saved_mayhem_builds, save_mayhem_build,
};
use crate::services::lcu_client::LcuClient;
use crate::services::live_game::{fetch_live_game_state, LiveGameState};
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

/// Returns whether the LCU API / WebSocket is currently connected and communicating.
#[tauri::command]
pub async fn api_is_lcu_connected() -> bool {
    if crate::services::lcu_websocket::LcuWsWatcher::is_connected() {
        return true;
    }
    if let Some(creds) = LcuClient::find_credentials() {
        if let Ok(client) = LcuClient::new(creds) {
            return client
                .get_json_response::<Value>("/lol-gameflow/v1/gameflow-phase")
                .await
                .is_ok();
        }
    }
    false
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

/// Computes or retrieves from cache the full build analysis for a champion in ARAM Mayhem.
#[tauri::command]
pub async fn api_mayhem_champion_build(
    champion_id: i64,
    augment_ids: Option<Vec<String>>,
    algorithm_version: Option<String>,
) -> Result<MayhemChampionBuildSummary, String> {
    compute_mayhem_champion_build(
        champion_id,
        augment_ids.unwrap_or_default(),
        algorithm_version,
    )
    .await
}

/// Batch-computes or retrieves builds for multiple champions (team roster and bench).
#[tauri::command]
pub async fn api_mayhem_team_builds(
    champion_ids: Vec<i64>,
    algorithm_version: Option<String>,
) -> Result<HashMap<i64, MayhemChampionBuildSummary>, String> {
    let mut results = HashMap::new();
    for id in champion_ids {
        if id <= 0 {
            continue;
        }
        if let Ok(summary) =
            compute_mayhem_champion_build(id, Vec::new(), algorithm_version.clone()).await
        {
            results.insert(id, summary);
        }
    }
    Ok(results)
}

/// Persists a mined build (e.g. when a match ends).
#[tauri::command]
pub async fn api_save_mayhem_build(
    payload: SaveMayhemBuildPayload,
) -> Result<SavedMayhemBuild, String> {
    save_mayhem_build(payload).await
}

/// Lists saved builds, optionally for one champion.
#[tauri::command]
pub async fn api_list_saved_mayhem_builds(
    champion_id: Option<i64>,
) -> Result<Vec<SavedMayhemBuild>, String> {
    list_saved_mayhem_builds(champion_id).await
}

/// Deletes a saved build by id.
#[tauri::command]
pub async fn api_delete_saved_mayhem_build(id: String) -> Result<(), String> {
    delete_saved_mayhem_build(id).await
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

/// Swaps to an available champion in the active champ select session (either from the bench or by requesting a swap from a teammate).
#[tauri::command]
pub async fn api_champ_select_swap(champion_id: i64) -> Result<bool, String> {
    let Some(creds) = LcuClient::find_credentials() else {
        return Err("League Client is not running".to_string());
    };
    let Ok(client) = LcuClient::new(creds) else {
        return Err("Failed to create LCU client".to_string());
    };

    let session: Value = client
        .get_json("/lol-champ-select/v1/session")
        .await
        .map_err(|e| format!("Champ select session not available: {e}"))?;

    let is_on_bench = session
        .get("benchChampions")
        .and_then(Value::as_array)
        .map(|bench| {
            bench
                .iter()
                .any(|b| b.get("championId").and_then(Value::as_i64) == Some(champion_id))
        })
        .unwrap_or(false);

    if is_on_bench {
        let path = format!("/lol-champ-select/v1/session/bench/swap/{champion_id}");
        let status = client.post_json_status(&path, &json!({})).await?;
        return Ok(status >= 200 && status < 300);
    }

    let local_cell_id = session
        .get("localPlayerCellId")
        .and_then(Value::as_i64)
        .unwrap_or(-1);
    let teammate_cell_id = session
        .get("myTeam")
        .and_then(Value::as_array)
        .and_then(|team| {
            team.iter().find_map(|m| {
                let cell_id = m.get("cellId").and_then(Value::as_i64)?;
                let c_id = m.get("championId").and_then(Value::as_i64)?;
                if cell_id != local_cell_id && c_id == champion_id {
                    Some(cell_id)
                } else {
                    None
                }
            })
        });

    if let Some(cell_id) = teammate_cell_id {
        let path = format!("/lol-champ-select/v1/session/champion-swaps/{cell_id}/request");
        let status = client.post_json_status(&path, &json!({})).await?;
        return Ok(status >= 200 && status < 300);
    }

    Err(format!(
        "Champion {champion_id} is not available in bench or team roster"
    ))
}

#[tauri::command]
pub async fn api_get_live_game_state() -> Result<LiveGameState, String> {
    fetch_live_game_state().await
}

#[tauri::command]
pub async fn api_get_live_build_advice(
    champion_id: i64,
    strategy_id: Option<String>,
    current_gold: f64,
    inventory_item_ids: Vec<i64>,
    level: Option<u32>,
    game_time: Option<f64>,
    augment_ids: Option<Vec<String>>,
) -> Result<MayhemLiveAdvice, String> {
    let summary =
        compute_mayhem_champion_build(champion_id, augment_ids.unwrap_or_default(), None).await?;
    let strat = if let Some(ref sid) = strategy_id {
        summary.strategies.iter().find(|s| &s.id == sid)
    } else {
        None
    }
    .or_else(|| summary.strategies.first())
    .ok_or_else(|| "No build strategy found".to_string())?;

    Ok(compute_live_build_advice(
        strat,
        current_gold,
        &inventory_item_ids,
        level.unwrap_or(1),
        game_time.unwrap_or(0.0),
    ))
}

#[tauri::command]
pub fn api_sync_overlay_window(
    app: tauri::AppHandle,
    enabled: bool,
    interactive: bool,
) -> Result<crate::services::live_overlay::GameWindowRect, String> {
    crate::services::live_overlay::sync_overlay_window(&app, enabled, interactive)
}

#[tauri::command]
pub async fn api_scan_live_augment_cards(
    app: tauri::AppHandle,
    champion_id: Option<i64>,
    with_augment_ids: Option<Vec<String>>,
) -> Result<Vec<crate::models::history::OverlayAugmentCard>, String> {
    use tauri::Emitter;
    let cards = crate::services::live_overlay::scan_in_game_augment_cards(
        champion_id,
        with_augment_ids.unwrap_or_default(),
    )
    .await?;

    crate::services::live_overlay::update_overlay_cards(&cards);
    let _ = app.emit("overlay:cards-updated", &cards);
    Ok(cards)
}

#[tauri::command]
pub fn api_emit_overlay_state(
    app: tauri::AppHandle,
    payload: Option<serde_json::Value>,
    state: Option<serde_json::Value>,
) -> Result<(), String> {
    use tauri::Emitter;
    let val = payload.or(state).unwrap_or(serde_json::Value::Null);
    crate::services::live_overlay::notify_overlay_state_from_frontend(&val);
    app.emit("overlay:state-sync", &val)
        .map_err(|e| e.to_string())
}
