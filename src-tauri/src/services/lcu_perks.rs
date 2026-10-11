//! # LCU Perks & Companion Slot Service
//!
//! Manages automatic `[LCU COMPANION SLOT]` creation, resolution,
//! and exporting saved Companion rune pages directly into the League Client.

use crate::services::lcu_client::LcuClient;
use crate::services::rune_store::RuneStore;
use serde_json::{json, Value};

/// Default perk IDs used when creating or initializing a blank companion slot.
pub const DEFAULT_PERKS: [i64; 9] = [8005, 9111, 9104, 8299, 8126, 8135, 5008, 5008, 5002];

/// Checks whether a companion slot (`[LCU COMPANION SLOT]` or `[LCC] *`) exists in the League Client.
/// If not, attempts to create one or rename the oldest/first editable page.
pub async fn ensure_companion_slot(client: &LcuClient) -> Result<Option<i64>, String> {
    let pages_val: Value = match client.get_json("/lol-perks/v1/pages").await {
        Ok(v) => v,
        Err(e) => return Err(format!("Failed to query /lol-perks/v1/pages: {}", e)),
    };

    let pages_arr = match pages_val.as_array() {
        Some(arr) => arr,
        None => return Ok(None),
    };

    // 1. Check if a companion slot page already exists
    for page in pages_arr {
        let name = page.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let id = page.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let is_editable = page
            .get("isEditable")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if is_editable
            && (name == "[LCU COMPANION SLOT]"
                || name.starts_with("[LCUC]")
                || name.starts_with("[LCC]"))
        {
            return Ok(Some(id));
        }
    }

    // 2. Try creating a new companion slot page
    let create_body = json!({
        "name": "[LCU COMPANION SLOT]",
        "primaryStyleId": 8000,
        "subStyleId": 8100,
        "selectedPerkIds": DEFAULT_PERKS,
        "isEditable": true,
        "isActive": true
    });

    if let Ok(new_page) = client
        .post_json::<Value>("/lol-perks/v1/pages", &create_body)
        .await
    {
        if let Some(new_id) = new_page.get("id").and_then(|v| v.as_i64()) {
            return Ok(Some(new_id));
        }
    }

    // 3. Fallback: If max page capacity is reached, find the first editable client page to convert
    for page in pages_arr {
        let is_editable = page
            .get("isEditable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let id = page.get("id").and_then(|v| v.as_i64()).unwrap_or(0);

        if is_editable && id > 0 {
            let update_body = json!({
                "id": id,
                "name": "[LCU COMPANION SLOT]",
                "isEditable": true,
                "isActive": true
            });
            let _ = client
                .put_json_status(&format!("/lol-perks/v1/pages/{}", id), &update_body)
                .await;
            return Ok(Some(id));
        }
    }

    Ok(None)
}

/// Exports a saved Companion rune set from the SQLite database into the League Client.
///
/// Updates the client slot with `[LCUC] <rune_page_name>` and sets it as the equipped/active page.
pub async fn export_runeset_to_client(runeset_id: i64) -> Result<i64, String> {
    let client = LcuClient::connect()?;
    let store = RuneStore::new()?;
    let record = store
        .get_runeset(runeset_id)?
        .ok_or_else(|| format!("Runeset with ID {} not found in database.", runeset_id))?;

    let export_name = format!("[LCUC] {}", record.rune_page_name);
    let primary_style_id = record.primary_style_id.unwrap_or(8000);
    let sub_style_id = record.sub_style_id.unwrap_or(8100);
    let mut selected_perks = record.selected_perk_ids;

    while selected_perks.len() < 9 {
        if selected_perks.len() < DEFAULT_PERKS.len() {
            selected_perks.push(DEFAULT_PERKS[selected_perks.len()]);
        } else {
            selected_perks.push(5008);
        }
    }

    let pages_val: Value = client
        .get_json("/lol-perks/v1/pages")
        .await
        .unwrap_or(json!([]));
    let pages_arr = pages_val.as_array();

    // 1. Look for existing companion slot or [LCUC] / [LCC] page
    let mut target_slot_id: Option<i64> = None;

    if let Some(arr) = pages_arr {
        for page in arr {
            let name = page.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let is_editable = page
                .get("isEditable")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let id = page.get("id").and_then(|v| v.as_i64()).unwrap_or(0);

            if is_editable
                && (name == "[LCU COMPANION SLOT]"
                    || name.starts_with("[LCUC]")
                    || name.starts_with("[LCC]"))
            {
                target_slot_id = Some(id);
                break;
            }
        }

        // If none found, look for first editable page
        if target_slot_id.is_none() {
            for page in arr {
                let is_editable = page
                    .get("isEditable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let id = page.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                if is_editable && id > 0 {
                    target_slot_id = Some(id);
                    break;
                }
            }
        }
    }

    let exported_id = if let Some(id) = target_slot_id {
        // Update existing page via PUT /lol-perks/v1/pages/{id}
        let update_body = json!({
            "id": id,
            "name": export_name,
            "primaryStyleId": primary_style_id,
            "subStyleId": sub_style_id,
            "selectedPerkIds": selected_perks,
            "isEditable": true,
            "isActive": true
        });

        match client
            .put_json_status(&format!("/lol-perks/v1/pages/{}", id), &update_body)
            .await
        {
            Ok(_) => id,
            Err(e) => return Err(format!("Failed updating rune page {}: {}", id, e)),
        }
    } else {
        // Create new page via POST /lol-perks/v1/pages
        let create_body = json!({
            "name": export_name,
            "primaryStyleId": primary_style_id,
            "subStyleId": sub_style_id,
            "selectedPerkIds": selected_perks,
            "isEditable": true,
            "isActive": true
        });

        match client
            .post_json::<Value>("/lol-perks/v1/pages", &create_body)
            .await
        {
            Ok(new_page) => new_page.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
            Err(e) => return Err(format!("Failed creating exported rune page: {}", e)),
        }
    };

    if exported_id > 0 {
        // Ensure page is set to current active in the client
        let _ = client
            .put_json_status("/lol-perks/v1/currentpage", &json!({ "id": exported_id }))
            .await;
    }

    Ok(exported_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_perks_length() {
        assert_eq!(DEFAULT_PERKS.len(), 9);
    }
}
