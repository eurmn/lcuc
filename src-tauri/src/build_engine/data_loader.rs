use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RawParticipantMatch {
    pub win: bool,
    pub items: Vec<u32>,
    pub augments: Vec<String>,
}

fn database_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu-companion")
}

fn parse_items(raw: &str) -> Vec<u32> {
    if let Ok(nums) = serde_json::from_str::<Vec<i64>>(raw) {
        nums.into_iter()
            .map(|id| id as u32)
            .filter(|&id| crate::build_engine::item_catalog::is_valid_build_item(id))
            .collect()
    } else {
        Vec::new()
    }
}

fn parse_augments(raw: &str) -> Vec<String> {
    if let Ok(vals) = serde_json::from_str::<Vec<serde_json::Value>>(raw) {
        vals.into_iter()
            .filter_map(|v| match v {
                serde_json::Value::String(s) => {
                    let t = s.trim().to_string();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t)
                    }
                }
                serde_json::Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect()
    } else {
        Vec::new()
    }
}

fn load_from_sqlite(
    path: &Path,
    table: &str,
    champion_id: u32,
) -> Result<Vec<RawParticipantMatch>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("Failed to open DB {:?}: {}", path, e))?;

    let _ = conn.busy_timeout(std::time::Duration::from_millis(3000));

    let query = format!(
        "SELECT win, item_ids_json, augment_ids_json FROM {} WHERE champion_id = ?1",
        table
    );
    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(_) => return Ok(Vec::new()),
    };

    let rows = stmt
        .query_map([champion_id], |row| {
            let win: i64 = row.get(0)?;
            let items_json: String = row.get(1)?;
            let augs_json: String = row.get(2)?;
            Ok((win != 0, items_json, augs_json))
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    let mut matches = Vec::new();
    for row in rows.flatten() {
        let (win, items_raw, augs_raw) = row;
        matches.push(RawParticipantMatch {
            win,
            items: parse_items(&items_raw),
            augments: parse_augments(&augs_raw),
        });
    }

    Ok(matches)
}

pub fn load_champion_matches(champion_id: u32) -> Result<Vec<RawParticipantMatch>, String> {
    let dir = database_dir();
    let global_db = dir.join("global-aram.db");
    let agent_db = dir.join("agent.db");

    let global_matches = load_from_sqlite(&global_db, "aram_participants", champion_id)?;
    if !global_matches.is_empty() {
        return Ok(global_matches);
    }

    load_from_sqlite(&agent_db, "game_build_facts", champion_id)
}
