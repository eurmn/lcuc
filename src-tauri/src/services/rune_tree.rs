//! # Rune Tree & CDragon Caching Engine
//!
//! Fetches and caches perk metadata (`perks.json`) and style trees (`perkstyles.json`)
//! from CommunityDragon (CDragon) and synthesizes normalized tree models for UI rendering.

use crate::models::{
    ClientRunePageSummary, RuneOption, RuneSlot, RuneStyleTree, RuneTreePayload,
};
use crate::services::asset_cache::get_cached_asset_data_uri;
use crate::services::lcu_client::LcuClient;
use crate::services::rune_store::RuneStore;
use lazy_static::lazy_static;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

const CDRAGON_BASE: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default";
const PERKS_URL: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perks.json";
const PERKSTYLES_URL: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perkstyles.json";

lazy_static! {
    static ref PERKS_CACHE: RwLock<HashMap<i64, Value>> = RwLock::new(HashMap::new());
    static ref PERKSTYLES_CACHE: RwLock<HashMap<i64, Value>> = RwLock::new(HashMap::new());
    static ref CACHE_INITIALIZED: RwLock<bool> = RwLock::new(false);
}

/// Converts an internal game asset path to a public CommunityDragon CDN URL.
fn icon_path_to_url(icon_path: &str) -> String {
    let prefix = "/lol-game-data/assets";
    let normalized = icon_path.trim().to_lowercase();
    if let Some(stripped) = normalized.strip_prefix(prefix) {
        format!("{}{}", CDRAGON_BASE, stripped)
    } else if normalized.starts_with('/') {
        format!("{}{}", CDRAGON_BASE, normalized)
    } else {
        format!("{}/{}", CDRAGON_BASE, normalized)
    }
}

/// Ensures that the CommunityDragon perk metadata and style definitions are loaded into memory.
///
/// Lazily downloads `perks.json` and `perkstyles.json` on the first call and stores them in
/// thread-safe asynchronous read-write locks (`RwLock`).
pub async fn ensure_cdragon_cache() -> Result<(), String> {
    {
        let initialized = CACHE_INITIALIZED.read().await;
        if *initialized {
            return Ok(());
        }
    }

    let client = reqwest::Client::new();
    let perks_resp: Value = client
        .get(PERKS_URL)
        .send()
        .await
        .map_err(|e| format!("Failed fetching perks.json: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed parsing perks.json: {}", e))?;

    let styles_resp: Value = client
        .get(PERKSTYLES_URL)
        .send()
        .await
        .map_err(|e| format!("Failed fetching perkstyles.json: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed parsing perkstyles.json: {}", e))?;

    let mut perks_map = PERKS_CACHE.write().await;
    if let Some(arr) = perks_resp.as_array() {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|v| v.as_i64()) {
                perks_map.insert(id, item.clone());
            }
        }
    }

    let mut styles_map = PERKSTYLES_CACHE.write().await;
    if let Some(styles_arr) = styles_resp.get("styles").and_then(|s| s.as_array()) {
        for style in styles_arr {
            if let Some(id) = style.get("id").and_then(|v| v.as_i64()) {
                styles_map.insert(id, style.clone());
            }
        }
    }

    let mut initialized = CACHE_INITIALIZED.write().await;
    *initialized = true;
    Ok(())
}

/// Resolves the name and icon URL for a given perk style ID (e.g. `8000` for Precision).
pub async fn get_style_info(style_id: Option<i64>) -> (String, String) {
    let id = match style_id {
        Some(i) => i,
        None => return ("".to_string(), "".to_string()),
    };

    let styles = PERKSTYLES_CACHE.read().await;
    if let Some(style) = styles.get(&id) {
        let name = style
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let icon_path = style
            .get("iconPath")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let icon_url = if !icon_path.is_empty() {
            let raw_url = icon_path_to_url(icon_path);
            get_cached_asset_data_uri(&raw_url).await
        } else {
            "".to_string()
        };
        (name, icon_url)
    } else {
        (format!("Style {}", id), "".to_string())
    }
}

/// Resolves the name and icon URL for a given perk ID (e.g. `8010` for Conqueror).
pub async fn get_perk_info(perk_id: i64) -> (String, String) {
    let perks = PERKS_CACHE.read().await;
    if let Some(perk_obj) = perks.get(&perk_id) {
        let name = perk_obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let icon_path = perk_obj
            .get("iconPath")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let icon_url = if !icon_path.is_empty() {
            let raw_url = icon_path_to_url(icon_path);
            get_cached_asset_data_uri(&raw_url).await
        } else {
            "".to_string()
        };
        (name, icon_url)
    } else {
        (format!("Perk {}", perk_id), "".to_string())
    }
}

/// Resolves which style ID a given keystone/perk belongs to.
pub async fn get_perk_style_id(perk_id: i64) -> Option<i64> {
    let styles = PERKSTYLES_CACHE.read().await;
    for (style_id, style_val) in styles.iter() {
        if let Some(slots_arr) = style_val.get("slots").and_then(|s| s.as_array()) {
            for slot in slots_arr {
                if let Some(perks_arr) = slot.get("perks").and_then(|p| p.as_array()) {
                    if perks_arr.iter().any(|p| p.as_i64() == Some(perk_id)) {
                        return Some(*style_id);
                    }
                }
            }
        }
    }
    None
}

/// Returns the natural in-game perk order for a style: (keystones, [row1, row2, row3])
pub async fn get_style_perk_order(style_id: i64) -> (Vec<i64>, Vec<Vec<i64>>) {
    let styles = PERKSTYLES_CACHE.read().await;
    let mut keystones = Vec::new();
    let mut rows = Vec::new();

    if let Some(style) = styles.get(&style_id) {
        if let Some(slots_val) = style.get("slots").and_then(|v| v.as_array()) {
            for slot in slots_val {
                let slot_type = slot.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if slot_type == "kKeyStone" {
                    if let Some(perks_arr) = slot.get("perks").and_then(|v| v.as_array()) {
                        for p in perks_arr {
                            if let Some(id) = p.as_i64() {
                                keystones.push(id);
                            }
                        }
                    }
                } else if slot_type != "kStatMod" && rows.len() < 3 {
                    let mut row_perks = Vec::new();
                    if let Some(perks_arr) = slot.get("perks").and_then(|v| v.as_array()) {
                        for p in perks_arr {
                            if let Some(id) = p.as_i64() {
                                row_perks.push(id);
                            }
                        }
                    }
                    if !row_perks.is_empty() {
                        rows.push(row_perks);
                    }
                }
            }
        }
    }

    (keystones, rows)
}



/// Builds a structured style tree with slots and selected options.
async fn build_style_tree(
    style_id: i64,
    selected_perks: &HashSet<i64>,
    is_secondary: bool,
) -> Option<RuneStyleTree> {
    let styles = PERKSTYLES_CACHE.read().await;
    let perks = PERKS_CACHE.read().await;

    let style = styles.get(&style_id)?;
    let style_name = style
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let icon_path = style
        .get("iconPath")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let style_icon_url = if !icon_path.is_empty() {
        let raw_url = icon_path_to_url(icon_path);
        get_cached_asset_data_uri(&raw_url).await
    } else {
        "".to_string()
    };

    let slots_val = style.get("slots")?.as_array()?;
    let mut keystone_slots = Vec::new();
    let mut regular_slots = Vec::new();

    for slot in slots_val {
        let slot_type = slot.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if slot_type == "kStatMod" {
            continue;
        }
        if slot_type == "kKeyStone" {
            keystone_slots.push(slot);
        } else {
            regular_slots.push(slot);
        }
    }

    let picked_slots: Vec<&Value> = if is_secondary {
        regular_slots.into_iter().take(3).collect()
    } else {
        let mut combined = Vec::new();
        if let Some(ks) = keystone_slots.first() {
            combined.push(*ks);
        }
        combined.extend(regular_slots.into_iter().take(3));
        combined
    };

    let mut rendered_slots = Vec::new();
    for slot in picked_slots {
        let label = slot
            .get("slotLabel")
            .and_then(|v| v.as_str())
            .unwrap_or("Slot")
            .to_string();
        let mut options = Vec::new();

        if let Some(perks_arr) = slot.get("perks").and_then(|v| v.as_array()) {
            for p_val in perks_arr {
                if let Some(perk_id) = p_val.as_i64() {
                    let perk_obj = perks.get(&perk_id);
                    let perk_name = perk_obj
                        .and_then(|p| p.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let perk_icon_path = perk_obj
                        .and_then(|p| p.get("iconPath"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let perk_icon_url = if !perk_icon_path.is_empty() {
                        let raw_url = icon_path_to_url(perk_icon_path);
                        get_cached_asset_data_uri(&raw_url).await
                    } else {
                        "".to_string()
                    };

                    options.push(RuneOption {
                        id: perk_id,
                        name: perk_name,
                        icon_url: perk_icon_url,
                        selected: selected_perks.contains(&perk_id),
                    });
                }
            }
        }

        rendered_slots.push(RuneSlot { label, options });
    }

    Some(RuneStyleTree {
        id: style_id,
        name: style_name,
        icon_url: style_icon_url,
        slots: rendered_slots,
    })
}

/// Builds the 3 rows of stat modification rune slots.
async fn build_stat_mod_slots(
    selected_stat_mods: &[i64],
    primary_style_id: i64,
) -> Vec<RuneSlot> {
    let styles = PERKSTYLES_CACHE.read().await;
    let perks = PERKS_CACHE.read().await;

    let style = match styles.get(&primary_style_id) {
        Some(s) => s,
        None => match styles.get(&8000) {
            Some(s) => s,
            None => return Vec::new(),
        },
    };

    let slots_val = match style.get("slots").and_then(|v| v.as_array()) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let stat_slots: Vec<&Value> = slots_val
        .iter()
        .filter(|s| s.get("type").and_then(|v| v.as_str()).unwrap_or("") == "kStatMod")
        .collect();

    let stat_order = [
        [5008, 5005, 5007],
        [5008, 5010, 5001],
        [5011, 5013, 5001],
    ];

    let mut rendered = Vec::new();
    for (row_idx, slot) in stat_slots.iter().enumerate() {
        let label = slot
            .get("slotLabel")
            .and_then(|v| v.as_str())
            .unwrap_or("Stat")
            .to_string();
        let expected_perk_id = selected_stat_mods.get(row_idx).copied();
        let mut options = Vec::new();

        if let Some(perks_arr) = slot.get("perks").and_then(|v| v.as_array()) {
            for p_val in perks_arr {
                if let Some(perk_id) = p_val.as_i64() {
                    let perk_obj = perks.get(&perk_id);
                    let perk_name = perk_obj
                        .and_then(|p| p.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let perk_icon_path = perk_obj
                        .and_then(|p| p.get("iconPath"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let perk_icon_url = if !perk_icon_path.is_empty() {
                        let raw_url = icon_path_to_url(perk_icon_path);
                        get_cached_asset_data_uri(&raw_url).await
                    } else {
                        "".to_string()
                    };

                    let is_selected = expected_perk_id.map(|exp| exp == perk_id).unwrap_or(false);

                    options.push(RuneOption {
                        id: perk_id,
                        name: perk_name,
                        icon_url: perk_icon_url,
                        selected: is_selected,
                    });
                }
            }
        }

        if let Some(order) = stat_order.get(row_idx) {
            options.sort_by_key(|o| order.iter().position(|&id| id == o.id).unwrap_or(999));
        }

        rendered.push(RuneSlot { label, options });
    }

    rendered
}

/// Builds a complete [`RuneTreePayload`] given page name, primary/sub styles, and selected perk IDs.
pub async fn build_rune_tree_payload_from_data(
    page_name: String,
    primary_style_id: Option<i64>,
    sub_style_id: Option<i64>,
    selected_perks: Vec<i64>,
    runeset_id: Option<i64>,
    page_id: Option<i64>,
    champion_id: Option<i64>,
    role: Option<u32>,
    is_editing: bool,
) -> RuneTreePayload {
    let selected_set: HashSet<i64> = selected_perks.iter().cloned().collect();
    let stat_mod_ids = if selected_perks.len() >= 9 {
        selected_perks[6..9].to_vec()
    } else if selected_perks.len() >= 3 {
        selected_perks[selected_perks.len() - 3..].to_vec()
    } else {
        Vec::new()
    };

    let stat_style_id = primary_style_id.unwrap_or(8000);

    let primary = if let Some(pid) = primary_style_id {
        build_style_tree(pid, &selected_set, false).await
    } else {
        None
    };

    let secondary = if let Some(sid) = sub_style_id {
        build_style_tree(sid, &selected_set, true).await
    } else {
        None
    };

    let stat_mods = build_stat_mod_slots(&stat_mod_ids, stat_style_id).await;

    RuneTreePayload {
        page_name,
        primary,
        secondary,
        stat_mods,
        runeset_id,
        page_id,
        primary_style_id,
        sub_style_id,
        selected_perk_ids: selected_perks,
        champion_id,
        role,
        is_editing,
    }
}

/// Updates the list of 9 selected perks when a user clicks a perk in edit mode.
pub async fn update_selected_perks(
    current_perks: &[i64],
    clicked_perk_id: i64,
    slot_type: &str,
    slot_index: usize,
    _primary_style_id: Option<i64>,
    sub_style_id: Option<i64>,
) -> Vec<i64> {
    ensure_cdragon_cache().await.ok();
    let mut perks = current_perks.to_vec();
    while perks.len() < 9 {
        perks.push(0);
    }

    match slot_type {
        "primary" => {
            if slot_index < 4 {
                perks[slot_index] = clicked_perk_id;
            }
        }
        "stat" => {
            if slot_index < 3 {
                perks[6 + slot_index] = clicked_perk_id;
            }
        }
        "secondary" => {
            let styles = PERKSTYLES_CACHE.read().await;
            let sub_style = sub_style_id.and_then(|sid| styles.get(&sid));

            let mut clicked_sub_slot = slot_index;
            if let Some(style) = sub_style {
                if let Some(slots_arr) = style.get("slots").and_then(|s| s.as_array()) {
                    let regular_slots: Vec<&Value> = slots_arr
                        .iter()
                        .filter(|s| {
                            s.get("type").and_then(|v| v.as_str()).unwrap_or("") != "kKeyStone"
                                && s.get("type").and_then(|v| v.as_str()).unwrap_or("") != "kStatMod"
                        })
                        .collect();

                    for (s_idx, s) in regular_slots.iter().enumerate() {
                        if let Some(perks_arr) = s.get("perks").and_then(|p| p.as_array()) {
                            if perks_arr.iter().any(|p| p.as_i64() == Some(clicked_perk_id)) {
                                clicked_sub_slot = s_idx;
                                break;
                            }
                        }
                    }

                    let mut slot_4 = None;
                    let mut slot_5 = None;
                    for (s_idx, s) in regular_slots.iter().enumerate() {
                        if let Some(perks_arr) = s.get("perks").and_then(|p| p.as_array()) {
                            if perks_arr.iter().any(|p| p.as_i64() == Some(perks[4])) {
                                slot_4 = Some(s_idx);
                            }
                            if perks_arr.iter().any(|p| p.as_i64() == Some(perks[5])) {
                                slot_5 = Some(s_idx);
                            }
                        }
                    }

                    if slot_4 == Some(clicked_sub_slot) {
                        perks[4] = clicked_perk_id;
                    } else if slot_5 == Some(clicked_sub_slot) {
                        perks[5] = clicked_perk_id;
                    } else {
                        perks[4] = perks[5];
                        perks[5] = clicked_perk_id;
                    }
                } else {
                    perks[4] = perks[5];
                    perks[5] = clicked_perk_id;
                }
            } else {
                perks[4] = perks[5];
                perks[5] = clicked_perk_id;
            }
        }
        _ => {}
    }

    perks
}

/// Fetches all rune pages currently stored in the running League Client (`GET /lol-perks/v1/pages`).
pub async fn get_client_rune_pages() -> Result<Vec<ClientRunePageSummary>, String> {
    ensure_cdragon_cache().await?;
    let client = LcuClient::connect()?;
    let payload: Value = client.get_json("/lol-perks/v1/pages").await?;

    let mut pages = Vec::new();
    if let Some(arr) = payload.as_array() {
        for item in arr {
            let id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Do not show companion slot / exported pages alongside regular client pages
            if name == "[LCU COMPANION SLOT]" || name.starts_with("[LCUC]") || name.starts_with("[LCC]") {
                continue;
            }

            let is_active = item
                .get("isActive")
                .or_else(|| item.get("current"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let is_editable = item
                .get("isEditable")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let primary_style_id = item.get("primaryStyleId").and_then(|v| v.as_i64());
            let sub_style_id = item.get("subStyleId").and_then(|v| v.as_i64());

            let mut selected_perks = Vec::new();
            if let Some(perks_arr) = item.get("selectedPerkIds").and_then(|v| v.as_array()) {
                for p in perks_arr {
                    if let Some(p_int) = p.as_i64() {
                        selected_perks.push(p_int);
                    }
                }
            }

            let (style_name, style_icon_url) = get_style_info(primary_style_id).await;

            pages.push(ClientRunePageSummary {
                id,
                name: if name.is_empty() { format!("Page {}", id) } else { name },
                is_active,
                is_editable,
                primary_style_id,
                sub_style_id,
                primary_style_name: style_name,
                primary_style_icon_url: style_icon_url,
                selected_perk_ids: selected_perks,
            });
        }
    }

    Ok(pages)
}

/// Fetches the currently equipped/active rune page from the client (`GET /lol-perks/v1/currentpage`).
pub async fn get_current_rune_tree_payload() -> Result<RuneTreePayload, String> {
    ensure_cdragon_cache().await?;
    let client = LcuClient::connect()?;
    let page: Value = client.get_json("/lol-perks/v1/currentpage").await?;

    let page_name = page
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Current Rune Page")
        .to_string();
    let primary_style_id = page.get("primaryStyleId").and_then(|v| v.as_i64());
    let sub_style_id = page.get("subStyleId").and_then(|v| v.as_i64());
    let client_page_id = page.get("id").and_then(|v| v.as_i64());

    let mut selected_perks = Vec::new();
    if let Some(perks_arr) = page.get("selectedPerkIds").and_then(|v| v.as_array()) {
        for p in perks_arr {
            if let Some(p_int) = p.as_i64() {
                selected_perks.push(p_int);
            }
        }
    }

    Ok(build_rune_tree_payload_from_data(
        page_name,
        primary_style_id,
        sub_style_id,
        selected_perks,
        None,
        client_page_id,
        None,
        Some(1),
        false,
    )
    .await)
}

/// Fetches a specific rune page by its client ID and returns the structured tree payload.
pub async fn get_client_page_rune_tree_payload(page_id: i64) -> Result<RuneTreePayload, String> {
    ensure_cdragon_cache().await?;
    let client = LcuClient::connect()?;
    let page: Value = match client.get_json(&format!("/lol-perks/v1/page/{}", page_id)).await {
        Ok(p) => p,
        Err(_) => {
            let all_pages: Value = client.get_json("/lol-perks/v1/pages").await?;
            let mut matched = None;
            if let Some(arr) = all_pages.as_array() {
                for item in arr {
                    if item.get("id").and_then(|v| v.as_i64()) == Some(page_id) {
                        matched = Some(item.clone());
                        break;
                    }
                }
            }
            matched.ok_or_else(|| format!("League client rune page {} not found.", page_id))?
        }
    };

    let page_name = page
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&format!("Rune Page {}", page_id))
        .to_string();
    let primary_style_id = page.get("primaryStyleId").and_then(|v| v.as_i64());
    let sub_style_id = page.get("subStyleId").and_then(|v| v.as_i64());

    let mut selected_perks = Vec::new();
    if let Some(perks_arr) = page.get("selectedPerkIds").and_then(|v| v.as_array()) {
        for p in perks_arr {
            if let Some(p_int) = p.as_i64() {
                selected_perks.push(p_int);
            }
        }
    }

    Ok(build_rune_tree_payload_from_data(
        page_name,
        primary_style_id,
        sub_style_id,
        selected_perks,
        None,
        Some(page_id),
        None,
        Some(1),
        false,
    )
    .await)
}

/// Retrieves a saved rune page from the local SQLite store and resolves its CDragon tree structure.
pub async fn get_saved_runeset_tree_payload(runeset_id: i64) -> Result<RuneTreePayload, String> {
    ensure_cdragon_cache().await?;
    let store = RuneStore::new()?;
    let record = store
        .get_runeset(runeset_id)?
        .ok_or_else(|| format!("Saved rune page #{} not found in database.", runeset_id))?;

    let champ_id = if record.champion_id > 0 {
        Some(record.champion_id)
    } else {
        None
    };

    Ok(build_rune_tree_payload_from_data(
        record.rune_page_name,
        record.primary_style_id,
        record.sub_style_id,
        record.selected_perk_ids,
        Some(runeset_id),
        record.rune_page_id,
        champ_id,
        Some(record.role),
        false,
    )
    .await)
}

/// Returns the name of the exported companion rune set if the client's current active page is a companion slot.
pub async fn get_active_companion_rune_name() -> Option<String> {
    let client = LcuClient::connect().ok()?;
    let page: Value = client.get_json("/lol-perks/v1/currentpage").await.ok()?;
    let name = page.get("name").and_then(|v| v.as_str())?;
    if let Some(stripped) = name.strip_prefix("[LCUC] ") {
        Some(stripped.to_string())
    } else if let Some(stripped) = name.strip_prefix("[LCC] ") {
        Some(stripped.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_live_client_rune_pages() {
        let pages = get_client_rune_pages().await;
        println!("Live client rune pages: {:?}", pages);
        assert!(pages.is_ok(), "Expected valid rune pages from running League client");
    }
}
