//! # Friend Manager & Whitelist Service
//!
//! Provides batch friend removal while protecting entries listed in `whitelist.txt`
//! and logging audit entries to `purged.txt`.

use crate::services::lcu_client::LcuClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Result summary of a friend purge execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRemovalResult {
    /// Number of friends deleted.
    pub deleted: u32,
    /// Number of friends preserved due to whitelist match.
    pub kept: u32,
    /// Any errors encountered during individual removals.
    pub errors: Vec<String>,
}

/// Loads whitelisted friend names, IDs, or Riot IDs from `whitelist.txt`.
fn load_whitelist() -> HashSet<String> {
    let mut set = HashSet::new();
    let paths = [
        PathBuf::from("whitelist.txt"),
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lcu_companion")
            .join("whitelist.txt"),
    ];

    for path in paths {
        if let Ok(contents) = fs::read_to_string(&path) {
            for line in contents.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    set.insert(trimmed.to_string());
                }
            }
        }
    }
    set
}

/// Fetches the current League friend list and deletes friends not present in the whitelist.
///
/// Records every removed friend to `purged.txt` in CSV format (`displayName,id`).
///
/// # Endpoints Consulted
/// * `GET /lol-chat/v1/friends`
/// * `DELETE /lol-chat/v1/friends/{id}`
pub async fn remove_friends() -> Result<FriendRemovalResult, String> {
    let client = LcuClient::connect()?;
    let whitelist = load_whitelist();

    let friends_val: Value = client.get_json("/lol-chat/v1/friends").await?;
    let friends_arr = friends_val
        .as_array()
        .ok_or_else(|| "Invalid friends payload from LCU".to_string())?;

    let mut result = FriendRemovalResult {
        deleted: 0,
        kept: 0,
        errors: Vec::new(),
    };

    let purged_path = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu_companion")
        .join("purged.txt");

    for friend in friends_arr {
        let fid = friend
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = friend
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let game_name = friend
            .get("gameName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let game_tag = friend
            .get("gameTag")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let riot_id = if !game_name.is_empty() && !game_tag.is_empty() {
            format!("{}#{}", game_name, game_tag)
        } else {
            "".to_string()
        };

        if whitelist.contains(&fid) || whitelist.contains(&name) || whitelist.contains(&riot_id) {
            result.kept += 1;
            continue;
        }

        match client.delete_status(&format!("/lol-chat/v1/friends/{}", fid)).await {
            Ok(status) if status == 200 || status == 204 => {
                result.deleted += 1;
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&purged_path) {
                    let display = if !riot_id.is_empty() { riot_id } else { name };
                    let _ = writeln!(file, "{},{}", display, fid);
                }
            }
            Ok(status) => {
                result.errors.push(format!("Failed removing friend {}: status {}", fid, status));
            }
            Err(e) => {
                result.errors.push(format!("Error removing friend {}: {}", fid, e));
            }
        }
    }

    Ok(result)
}
