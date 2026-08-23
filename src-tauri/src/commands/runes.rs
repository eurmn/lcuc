//! # Runes Tauri Commands
//!
//! Exposes rune page listing, importing, deletion, active tree loading,
//! interactive perk editing, and database updating endpoints to Svelte via typed JSON IPC.

use crate::models::{
    RuneSetSummary, RuneTreePayload, RunesOverviewPayload, SaveRuneTreePayload, SavedRuneSetSummary,
    SelectPerkPayload,
};
use crate::services::champions::detect_champion_from_name;
use crate::services::lcu_perks::export_runeset_to_client;
use crate::services::rune_store::RuneStore;
use crate::services::rune_tree::{
    build_rune_tree_payload_from_data, get_active_companion_rune_name,
    get_client_page_rune_tree_payload, get_client_rune_pages, get_current_rune_tree_payload,
    get_saved_runeset_tree_payload, get_style_info, update_selected_perks,
};

/// Returns the dual overview of Client Rune Pages and Saved Companion Runesets,
/// along with the active companion slot name if equipped.
#[tauri::command]
pub async fn api_runes_overview() -> Result<RunesOverviewPayload, String> {
    let client_pages = get_client_rune_pages().await.unwrap_or_default();
    let raw_saved = match RuneStore::new() {
        Ok(store) => store.list_runesets(100).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let mut saved_runesets = Vec::with_capacity(raw_saved.len());
    for item in raw_saved {
        let (primary_style_name, primary_style_icon_url) =
            get_style_info(item.primary_style_id).await;
        saved_runesets.push(SavedRuneSetSummary {
            id: item.id,
            rune_page_name: item.rune_page_name,
            primary_style_id: item.primary_style_id,
            sub_style_id: item.sub_style_id,
            primary_style_name,
            primary_style_icon_url,
            champion_id: item.champion_id,
            source: item.source,
            created_at: item.created_at,
        });
    }

    let active_companion_name = get_active_companion_rune_name().await;

    Ok(RunesOverviewPayload {
        client_pages,
        saved_runesets,
        active_companion_name,
    })
}

/// Returns the list of saved companion rune pages.
#[tauri::command]
pub async fn api_runesets() -> Result<Vec<RuneSetSummary>, String> {
    let store = RuneStore::new()?;
    store.list_runesets(100)
}

/// Imports a client rune page into the local SQLite database and returns the new runeset ID.
#[tauri::command]
pub async fn api_import_client_page(
    page_id: i64,
    champion_id: Option<i64>,
) -> Result<i64, String> {
    let client_pages = get_client_rune_pages().await?;
    let target = client_pages
        .iter()
        .find(|p| p.id == page_id)
        .ok_or_else(|| format!("Client rune page {} not found", page_id))?;

    let resolved_champ_id = if let Some(cid) = champion_id {
        if cid > 0 {
            cid
        } else if let Some(detected) = detect_champion_from_name(&target.name).await {
            detected.id
        } else {
            0
        }
    } else if let Some(detected) = detect_champion_from_name(&target.name).await {
        detected.id
    } else {
        0
    };

    let store = RuneStore::new()?;
    let new_id = store.save_runeset(
        &target.name,
        target.primary_style_id,
        target.sub_style_id,
        &target.selected_perk_ids,
        None,
        resolved_champ_id,
        None,
        Some(target.id),
        "league_client",
    )?;

    Ok(new_id)
}

/// Exports a saved Companion rune set directly into the League Client as an equipped `[LCUC] <name>` page.
#[tauri::command]
pub async fn api_export_saved_rune_page(runeset_id: i64) -> Result<i64, String> {
    if runeset_id <= 0 {
        return Err("Invalid runeset ID".to_string());
    }
    export_runeset_to_client(runeset_id).await
}

/// Deletes a saved rune page from SQLite.
#[tauri::command]
pub async fn api_delete_runeset(runeset_id: i64) -> Result<bool, String> {
    let store = RuneStore::new()?;
    store.delete_runeset(runeset_id)
}

/// Updates the associated champion of a saved runeset in SQLite.
#[tauri::command]
pub async fn api_update_runeset_champion(
    runeset_id: i64,
    champion_id: i64,
) -> Result<bool, String> {
    let store = RuneStore::new()?;
    let record = store
        .get_runeset(runeset_id)?
        .ok_or_else(|| format!("Runeset {} not found", runeset_id))?;

    store.update_runeset(
        runeset_id,
        &record.rune_page_name,
        record.primary_style_id,
        record.sub_style_id,
        &record.selected_perk_ids,
        Some(champion_id),
    )
}

/// Returns the initial rune tree payload, prioritizing the first saved companion runeset if present.
#[tauri::command]
pub async fn api_rune_tree_current() -> Result<RuneTreePayload, String> {
    if let Ok(store) = RuneStore::new() {
        if let Ok(saved) = store.list_runesets(1) {
            if let Some(first) = saved.first() {
                if let Ok(tree) = get_saved_runeset_tree_payload(first.id).await {
                    return Ok(tree);
                }
            }
        }
    }

    get_current_rune_tree_payload().await
}

/// Returns the rune tree for a specific client page.
#[tauri::command]
pub async fn api_rune_tree_client(page_id: i64) -> Result<RuneTreePayload, String> {
    get_client_page_rune_tree_payload(page_id).await
}

/// Returns the rune tree for a saved SQLite page.
#[tauri::command]
pub async fn api_rune_tree_saved(runeset_id: i64) -> Result<RuneTreePayload, String> {
    get_saved_runeset_tree_payload(runeset_id).await
}

/// Updates the active perk selection while editing and recalculates the updated tree payload.
#[tauri::command]
pub async fn api_rune_tree_select_perk(
    payload: SelectPerkPayload,
) -> Result<RuneTreePayload, String> {
    let resolved_name = payload
        .page_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Custom Rune Page".to_string());

    let updated_perks = update_selected_perks(
        &payload.current_perks,
        payload.clicked_perk_id,
        &payload.slot_type,
        payload.slot_index,
        payload.primary_style_id,
        payload.sub_style_id,
    )
    .await;

    let tree = build_rune_tree_payload_from_data(
        resolved_name,
        payload.primary_style_id,
        payload.sub_style_id,
        updated_perks,
        payload.runeset_id,
        payload.page_id,
        None,
        true,
    )
    .await;

    Ok(tree)
}

/// Persists the updated rune page into the SQLite database and returns the new/updated runeset ID.
#[tauri::command]
pub async fn api_save_rune_tree(payload: SaveRuneTreePayload) -> Result<i64, String> {
    let resolved_name = if !payload.page_name.trim().is_empty() {
        payload.page_name.trim().to_string()
    } else {
        "Saved Rune Page".to_string()
    };

    let store = RuneStore::new()?;
    let champion_id = payload.champion_id.unwrap_or(0);
    let mut saved_id: Option<i64> = None;

    if let Some(rid) = payload.runeset_id {
        if rid > 0 {
            store.update_runeset(
                rid,
                &resolved_name,
                payload.primary_style_id,
                payload.sub_style_id,
                &payload.selected_perks,
                Some(champion_id),
            )?;
            saved_id = Some(rid);
        }
    }

    if saved_id.is_none() {
        let new_id = store.save_runeset(
            &resolved_name,
            payload.primary_style_id,
            payload.sub_style_id,
            &payload.selected_perks,
            None,
            champion_id,
            None,
            payload.page_id,
            "companion_custom",
        )?;
        saved_id = Some(new_id);
    }

    saved_id.ok_or_else(|| "Failed to save rune page".to_string())
}

/// Retrieves the last selected rune key persisted in SQLite.
#[tauri::command]
pub async fn api_get_selected_rune_key() -> Result<Option<String>, String> {
    let store = RuneStore::new()?;
    store.get_selected_rune_key()
}

/// Persists the last selected rune key in SQLite.
#[tauri::command]
pub async fn api_set_selected_rune_key(rune_key: Option<String>) -> Result<(), String> {
    let store = RuneStore::new()?;
    store.set_selected_rune_key(rune_key.as_deref())
}

