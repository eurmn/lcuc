//! # Runes Tauri Commands
//!
//! Exposes rune page listing, importing, deletion, active tree loading,
//! interactive perk editing, database updating, and out-of-band sidebar synchronization endpoints to HTMX.

use crate::models::RuneSetSummary;
use crate::render::{render_rune_tree, render_runes_sidebar};
use crate::services::lcu_perks::export_runeset_to_client;
use crate::services::rune_store::RuneStore;
use crate::services::rune_tree::{
    build_rune_tree_payload_from_data, get_client_page_rune_tree_payload, get_client_rune_pages,
    get_current_rune_tree_payload, get_saved_runeset_tree_payload, update_selected_perks,
};

fn parse_perks(input: Option<String>) -> Vec<i64> {
    let s = match input {
        Some(val) if !val.trim().is_empty() => val,
        _ => return Vec::new(),
    };
    if s.trim().starts_with('[') {
        serde_json::from_str(&s).unwrap_or_default()
    } else {
        s.split(',')
            .filter_map(|part| part.trim().parse::<i64>().ok())
            .collect()
    }
}

/// Returns the dual-section rune pages sidebar partial (Client Pages + Companion Saved Pages).
///
/// # Arguments
/// * `selected_key` - Optional item key to highlight (e.g. `client:54` or `saved:2`).
#[tauri::command]
pub async fn ui_runesets(selected_key: Option<String>) -> String {
    let client_pages = get_client_rune_pages().await.unwrap_or_default();
    let saved_runesets = match RuneStore::new() {
        Ok(store) => store.list_runesets(100).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    render_runes_sidebar(
        &client_pages,
        &saved_runesets,
        selected_key.as_deref(),
        None,
    )
    .await
}

/// Imports a client rune page into the local SQLite database and returns the refreshed sidebar.
///
/// # Arguments
/// * `page_id` / `pageId` - The League Client rune page ID to import.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn ui_import_client_page(page_id: Option<i64>, pageId: Option<i64>) -> String {
    let target_page_id = page_id.or(pageId).unwrap_or(0);
    let mut new_id: Option<i64> = None;

    if let Ok(client_pages) = get_client_rune_pages().await {
        if let Some(target) = client_pages.iter().find(|p| p.id == target_page_id) {
            if let Ok(store) = RuneStore::new() {
                if let Ok(id) = store.save_runeset(
                    &target.name,
                    target.primary_style_id,
                    target.sub_style_id,
                    &target.selected_perk_ids,
                    None,
                    0,
                    None,
                    Some(target.id),
                    "league_client",
                ) {
                    new_id = Some(id);
                }
            }
        }
    }

    let client_pages = get_client_rune_pages().await.unwrap_or_default();
    let saved_runesets = match RuneStore::new() {
        Ok(store) => store.list_runesets(100).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let selected_key = new_id
        .map(|id| format!("saved:{}", id))
        .unwrap_or_else(|| format!("client:{}", target_page_id));

    render_runes_sidebar(
        &client_pages,
        &saved_runesets,
        Some(&selected_key),
        None,
    )
    .await
}

/// Imports a client rune page into SQLite directly from the rune tree canvas,
/// renders the newly saved rune tree (now with Edit button), and triggers an OOB sidebar update.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn ui_import_client_page_from_tree(page_id: Option<i64>, pageId: Option<i64>) -> String {
    let target_page_id = page_id.or(pageId).unwrap_or(0);
    let mut new_id: Option<i64> = None;

    if let Ok(client_pages) = get_client_rune_pages().await {
        if let Some(target) = client_pages.iter().find(|p| p.id == target_page_id) {
            if let Ok(store) = RuneStore::new() {
                if let Ok(id) = store.save_runeset(
                    &target.name,
                    target.primary_style_id,
                    target.sub_style_id,
                    &target.selected_perk_ids,
                    None,
                    0,
                    None,
                    Some(target.id),
                    "league_client",
                ) {
                    new_id = Some(id);
                }
            }
        }
    }

    let saved_id = new_id.unwrap_or(0);
    let tree_html = match get_saved_runeset_tree_payload(saved_id).await {
        Ok(tree) => render_rune_tree(Some(&tree), None),
        Err(_) => match get_client_page_rune_tree_payload(target_page_id).await {
            Ok(tree) => render_rune_tree(Some(&tree), None),
            Err(err) => render_rune_tree(None, Some(&err)),
        },
    };

    let client_pages = get_client_rune_pages().await.unwrap_or_default();
    let saved_runesets = match RuneStore::new() {
        Ok(store) => store.list_runesets(100).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let selected_key = format!("saved:{}", saved_id);
    let sidebar_html = render_runes_sidebar(
        &client_pages,
        &saved_runesets,
        Some(&selected_key),
        None,
    )
    .await;

    let sidebar_oob = sidebar_html.replacen(
        r##"<div id="saved-runes-container""##,
        r##"<div id="saved-runes-container" hx-swap-oob="outerHTML""##,
        1,
    );

    format!("{}\n{}", tree_html, sidebar_oob)
}

/// Exports a saved Companion rune set directly into the League Client as an equipped `[LCC] <name>` page,
/// then returns the refreshed tree payload and an Out-of-Band (`hx-swap-oob`) sidebar update.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn ui_export_saved_rune_page(runeset_id: Option<i64>, runesetId: Option<i64>) -> String {
    let target_runeset_id = runeset_id.or(runesetId).unwrap_or(0);

    if target_runeset_id > 0 {
        let _ = export_runeset_to_client(target_runeset_id).await;
    }

    let tree_html = match get_saved_runeset_tree_payload(target_runeset_id).await {
        Ok(tree) => render_rune_tree(Some(&tree), None),
        Err(err) => render_rune_tree(None, Some(&err)),
    };

    let client_pages = get_client_rune_pages().await.unwrap_or_default();
    let saved_runesets = match RuneStore::new() {
        Ok(store) => store.list_runesets(100).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let selected_key = format!("saved:{}", target_runeset_id);
    let sidebar_html = render_runes_sidebar(
        &client_pages,
        &saved_runesets,
        Some(&selected_key),
        None,
    )
    .await;

    let sidebar_oob = sidebar_html.replacen(
        r##"<div id="saved-runes-container""##,
        r##"<div id="saved-runes-container" hx-swap-oob="outerHTML""##,
        1,
    );

    format!("{}\n{}", tree_html, sidebar_oob)
}

/// Deletes a saved rune page from SQLite and returns the refreshed sidebar.
///
/// # Arguments
/// * `runeset_id` / `runesetId` - The SQLite record ID to delete.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn ui_delete_runeset(runeset_id: Option<i64>, runesetId: Option<i64>) -> String {
    let target_runeset_id = runeset_id.or(runesetId).unwrap_or(0);

    if let Ok(store) = RuneStore::new() {
        let _ = store.delete_runeset(target_runeset_id);
    }

    let client_pages = get_client_rune_pages().await.unwrap_or_default();
    let saved_runesets = match RuneStore::new() {
        Ok(store) => store.list_runesets(100).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    render_runes_sidebar(&client_pages, &saved_runesets, None, None).await
}

/// Returns the initial rune tree partial (`#current-rune-tree-container`), prioritizing the first saved companion runeset if present.
#[tauri::command]
pub async fn ui_rune_tree_current() -> String {
    if let Ok(store) = RuneStore::new() {
        if let Ok(saved) = store.list_runesets(1) {
            if let Some(first) = saved.first() {
                if let Ok(tree) = get_saved_runeset_tree_payload(first.id).await {
                    return render_rune_tree(Some(&tree), None);
                }
            }
        }
    }

    match get_current_rune_tree_payload().await {
        Ok(tree) => render_rune_tree(Some(&tree), None),
        Err(err) => render_rune_tree(None, Some(&err)),
    }
}

/// Returns the rune tree for a specific client page, alongside an Out-of-Band (`hx-swap-oob`) sidebar update.
///
/// # Arguments
/// * `page_id` / `pageId` - The client page ID to load.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn ui_rune_tree_client(page_id: Option<i64>, pageId: Option<i64>) -> String {
    let target_page_id = page_id.or(pageId).unwrap_or(0);

    let tree_html = match get_client_page_rune_tree_payload(target_page_id).await {
        Ok(tree) => render_rune_tree(Some(&tree), None),
        Err(err) => render_rune_tree(None, Some(&err)),
    };

    let client_pages = get_client_rune_pages().await.unwrap_or_default();
    let saved_runesets = match RuneStore::new() {
        Ok(store) => store.list_runesets(100).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let selected_key = format!("client:{}", target_page_id);
    let sidebar_html = render_runes_sidebar(
        &client_pages,
        &saved_runesets,
        Some(&selected_key),
        None,
    )
    .await;

    let sidebar_oob = sidebar_html.replacen(
        r##"<div id="saved-runes-container""##,
        r##"<div id="saved-runes-container" hx-swap-oob="outerHTML""##,
        1,
    );

    format!("{}\n{}", tree_html, sidebar_oob)
}

/// Returns the rune tree for a saved SQLite page, alongside an Out-of-Band (`hx-swap-oob`) sidebar update.
///
/// # Arguments
/// * `runeset_id` / `runesetId` - The SQLite record ID to load.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn ui_rune_tree_saved(runeset_id: Option<i64>, runesetId: Option<i64>) -> String {
    let target_runeset_id = runeset_id.or(runesetId).unwrap_or(0);

    let tree_html = match get_saved_runeset_tree_payload(target_runeset_id).await {
        Ok(tree) => render_rune_tree(Some(&tree), None),
        Err(err) => render_rune_tree(None, Some(&err)),
    };

    let client_pages = get_client_rune_pages().await.unwrap_or_default();
    let saved_runesets = match RuneStore::new() {
        Ok(store) => store.list_runesets(100).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let selected_key = format!("saved:{}", target_runeset_id);
    let sidebar_html = render_runes_sidebar(
        &client_pages,
        &saved_runesets,
        Some(&selected_key),
        None,
    )
    .await;

    let sidebar_oob = sidebar_html.replacen(
        r##"<div id="saved-runes-container""##,
        r##"<div id="saved-runes-container" hx-swap-oob="outerHTML""##,
        1,
    );

    format!("{}\n{}", tree_html, sidebar_oob)
}

/// Switches the current rune tree card into interactive Edit Mode.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn ui_rune_tree_edit_mode(
    runeset_id: Option<i64>,
    runesetId: Option<i64>,
    page_id: Option<i64>,
    pageId: Option<i64>,
    page_name: Option<String>,
    pageName: Option<String>,
    primary_style_id: Option<i64>,
    primaryStyleId: Option<i64>,
    sub_style_id: Option<i64>,
    subStyleId: Option<i64>,
    current_perks: Option<String>,
    currentPerks: Option<String>,
) -> String {
    let target_runeset_id = runeset_id.or(runesetId);
    let target_page_id = page_id.or(pageId);
    let mut resolved_name = page_name.or(pageName).unwrap_or_default();
    let mut resolved_primary = primary_style_id.or(primaryStyleId);
    let mut resolved_sub = sub_style_id.or(subStyleId);
    let mut resolved_perks = parse_perks(current_perks.or(currentPerks));

    if resolved_perks.is_empty() {
        if let Some(rid) = target_runeset_id {
            if let Ok(store) = RuneStore::new() {
                if let Ok(Some(record)) = store.get_runeset(rid) {
                    if resolved_name.is_empty() {
                        resolved_name = record.rune_page_name;
                    }
                    if resolved_primary.is_none() {
                        resolved_primary = record.primary_style_id;
                    }
                    if resolved_sub.is_none() {
                        resolved_sub = record.sub_style_id;
                    }
                    resolved_perks = record.selected_perk_ids;
                }
            }
        } else if let Some(pid) = target_page_id {
            if let Ok(tree) = get_client_page_rune_tree_payload(pid).await {
                if resolved_name.is_empty() {
                    resolved_name = tree.page_name;
                }
                if resolved_primary.is_none() {
                    resolved_primary = tree.primary_style_id;
                }
                if resolved_sub.is_none() {
                    resolved_sub = tree.sub_style_id;
                }
                resolved_perks = tree.selected_perk_ids;
            }
        }
    }

    if resolved_name.is_empty() {
        resolved_name = "Custom Rune Page".to_string();
    }

    let tree = build_rune_tree_payload_from_data(
        resolved_name,
        resolved_primary,
        resolved_sub,
        resolved_perks,
        target_runeset_id,
        target_page_id,
        true,
    )
    .await;

    render_rune_tree(Some(&tree), None)
}

/// Cancels edit mode and re-renders the rune tree in view mode.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn ui_rune_tree_cancel_edit(
    runeset_id: Option<i64>,
    runesetId: Option<i64>,
    page_id: Option<i64>,
    pageId: Option<i64>,
) -> String {
    let target_runeset_id = runeset_id.or(runesetId);
    let target_page_id = page_id.or(pageId);

    if let Some(rid) = target_runeset_id {
        match get_saved_runeset_tree_payload(rid).await {
            Ok(tree) => render_rune_tree(Some(&tree), None),
            Err(err) => render_rune_tree(None, Some(&err)),
        }
    } else if let Some(pid) = target_page_id {
        match get_client_page_rune_tree_payload(pid).await {
            Ok(tree) => render_rune_tree(Some(&tree), None),
            Err(err) => render_rune_tree(None, Some(&err)),
        }
    } else {
        match get_current_rune_tree_payload().await {
            Ok(tree) => render_rune_tree(Some(&tree), None),
            Err(err) => render_rune_tree(None, Some(&err)),
        }
    }
}

/// Updates the active perk selection while remaining in edit mode.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn ui_rune_tree_select_perk(
    runeset_id: Option<i64>,
    runesetId: Option<i64>,
    page_id: Option<i64>,
    pageId: Option<i64>,
    page_name: Option<String>,
    pageName: Option<String>,
    primary_style_id: Option<i64>,
    primaryStyleId: Option<i64>,
    sub_style_id: Option<i64>,
    subStyleId: Option<i64>,
    current_perks: Option<String>,
    currentPerks: Option<String>,
    clicked_perk_id: Option<i64>,
    clickedPerkId: Option<i64>,
    slot_type: Option<String>,
    slotType: Option<String>,
    slot_index: Option<usize>,
    slotIndex: Option<usize>,
) -> String {
    let target_runeset_id = runeset_id.or(runesetId);
    let target_page_id = page_id.or(pageId);
    let resolved_name = page_name
        .or(pageName)
        .unwrap_or_else(|| "Custom Rune Page".to_string());
    let resolved_primary = primary_style_id.or(primaryStyleId);
    let resolved_sub = sub_style_id.or(subStyleId);
    let current = parse_perks(current_perks.or(currentPerks));
    let clicked = clicked_perk_id.or(clickedPerkId).unwrap_or(0);
    let stype = slot_type.or(slotType).unwrap_or_default();
    let sidx = slot_index.or(slotIndex).unwrap_or(0);

    let updated_perks = update_selected_perks(
        &current,
        clicked,
        &stype,
        sidx,
        resolved_primary,
        resolved_sub,
    )
    .await;

    let tree = build_rune_tree_payload_from_data(
        resolved_name,
        resolved_primary,
        resolved_sub,
        updated_perks,
        target_runeset_id,
        target_page_id,
        true,
    )
    .await;

    render_rune_tree(Some(&tree), None)
}

/// Persists the updated rune page into the SQLite database and returns the view-mode tree with an OOB sidebar update.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn ui_save_rune_tree(
    runeset_id: Option<i64>,
    runesetId: Option<i64>,
    page_id: Option<i64>,
    pageId: Option<i64>,
    page_name: Option<String>,
    pageName: Option<String>,
    primary_style_id: Option<i64>,
    primaryStyleId: Option<i64>,
    sub_style_id: Option<i64>,
    subStyleId: Option<i64>,
    selected_perks: Option<String>,
    selectedPerks: Option<String>,
) -> String {
    let target_runeset_id = runeset_id.or(runesetId);
    let target_page_id = page_id.or(pageId);
    let resolved_name = page_name
        .or(pageName)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Saved Rune Page".to_string());
    let resolved_primary = primary_style_id.or(primaryStyleId);
    let resolved_sub = sub_style_id.or(subStyleId);
    let perks = parse_perks(selected_perks.or(selectedPerks));

    let mut saved_id: Option<i64> = None;

    if let Ok(store) = RuneStore::new() {
        if let Some(rid) = target_runeset_id {
            if rid > 0 {
                let _ = store.update_runeset(
                    rid,
                    &resolved_name,
                    resolved_primary,
                    resolved_sub,
                    &perks,
                );
                saved_id = Some(rid);
            }
        }

        if saved_id.is_none() {
            if let Ok(new_id) = store.save_runeset(
                &resolved_name,
                resolved_primary,
                resolved_sub,
                &perks,
                None,
                0,
                None,
                target_page_id,
                "companion_custom",
            ) {
                saved_id = Some(new_id);
            }
        }
    }

    let final_id = saved_id.unwrap_or(0);
    let tree_html = match get_saved_runeset_tree_payload(final_id).await {
        Ok(tree) => render_rune_tree(Some(&tree), None),
        Err(err) => render_rune_tree(None, Some(&err)),
    };

    let client_pages = get_client_rune_pages().await.unwrap_or_default();
    let saved_runesets = match RuneStore::new() {
        Ok(store) => store.list_runesets(100).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let selected_key = format!("saved:{}", final_id);
    let sidebar_html = render_runes_sidebar(
        &client_pages,
        &saved_runesets,
        Some(&selected_key),
        None,
    )
    .await;

    let sidebar_oob = sidebar_html.replacen(
        r##"<div id="saved-runes-container""##,
        r##"<div id="saved-runes-container" hx-swap-oob="outerHTML""##,
        1,
    );

    format!("{}\n{}", tree_html, sidebar_oob)
}

/// Direct JSON API command returning the list of saved rune pages.
#[tauri::command]
pub async fn api_runesets() -> Result<Vec<RuneSetSummary>, String> {
    let store = RuneStore::new()?;
    store.list_runesets(100)
}
