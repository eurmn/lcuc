use crate::agent_supervisor::{AgentStatus, AgentSupervisor};
use crate::logger::{self, Level};
use crate::services::history::{enrich_global_statistics, enrich_statistics_response};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;

static ENRICHED_CACHE_GENERATION: AtomicU64 = AtomicU64::new(0);

lazy_static::lazy_static! {
    static ref ENRICHED_DATASET_CACHE: RwLock<HashMap<String, (u64, Instant, serde_json::Value)>> =
        RwLock::new(HashMap::new());
}

pub fn invalidate_enriched_statistics_cache() {
    ENRICHED_CACHE_GENERATION.fetch_add(1, Ordering::SeqCst);
}

#[tauri::command]
pub async fn api_agent_status(
    supervisor: tauri::State<'_, AgentSupervisor>,
) -> Result<AgentStatus, String> {
    Ok(supervisor.status().await)
}

#[tauri::command]
pub async fn api_build_statistics(
    supervisor: tauri::State<'_, AgentSupervisor>,
) -> Result<serde_json::Value, String> {
    supervisor.request("build_statistics").await
}

#[tauri::command]
pub async fn api_statistics(
    query: serde_json::Value,
    supervisor: tauri::State<'_, AgentSupervisor>,
) -> Result<serde_json::Value, String> {
    let scope = query
        .get("scope")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("personal");
    let kind = query
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("champion");
    let hide_low_pickrate = query
        .get("hide_low_pickrate")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let search = query
        .get("search")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let sort_key = query
        .get("sort_key")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("wpa");
    let descending = query
        .get("descending")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let limit = query
        .get("limit")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(100) as usize;
    let offset = query
        .get("offset")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0) as usize;

    let cache_key = format!("{scope}:{kind}:{hide_low_pickrate}");
    let current_gen = ENRICHED_CACHE_GENERATION.load(Ordering::SeqCst);

    let cached_dataset = {
        let cache = ENRICHED_DATASET_CACHE.read().await;
        cache.get(&cache_key).and_then(|(gen, created_at, val)| {
            if *gen == current_gen && created_at.elapsed() < std::time::Duration::from_secs(600) {
                Some(val.clone())
            } else {
                None
            }
        })
    };

    let mut enriched = match cached_dataset {
        Some(val) => val,
        None => {
            let mut agent_query = query.clone();
            if let Some(obj) = agent_query.as_object_mut() {
                obj.insert(
                    "search".to_string(),
                    serde_json::Value::String(String::new()),
                );
                obj.insert("offset".to_string(), serde_json::Value::from(0));
                obj.insert("limit".to_string(), serde_json::Value::from(10000));
                obj.insert(
                    "sort_key".to_string(),
                    serde_json::Value::String("name".to_string()),
                );
                obj.insert("descending".to_string(), serde_json::Value::Bool(false));
            }

            let response = supervisor
                .request_with_args("statistics", agent_query)
                .await?;
            let enriched = enrich_statistics_response(response).await?;

            let mut cache = ENRICHED_DATASET_CACHE.write().await;
            cache.insert(cache_key, (current_gen, Instant::now(), enriched.clone()));
            enriched
        }
    };

    if let Some(rows) = enriched
        .get_mut("rows")
        .and_then(serde_json::Value::as_array_mut)
    {
        if !search.is_empty() {
            rows.retain(|row| {
                let matches_name = row
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains(&search);
                let matches_comp = row
                    .get("components")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|comps| {
                        comps.iter().any(|c| {
                            c.get("name")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("")
                                .to_ascii_lowercase()
                                .contains(&search)
                        })
                    });
                matches_name || matches_comp
            });
        }

        let sort_val = |row: &serde_json::Value| match sort_key {
            "games" => row
                .get("global_games")
                .or_else(|| row.get("games"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            "pickRate" => row
                .get("pick_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            "winRate" => row
                .get("global_win_rate")
                .or_else(|| row.get("win_rate"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            "personalWinRate" => row
                .get("personal_win_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            "globalWinRate" => row
                .get("global_win_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            "kda" => {
                let kills = row
                    .get("kills")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                let assists = row
                    .get("assists")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                let deaths = row
                    .get("deaths")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                (kills + assists) / deaths.max(1.0)
            }
            "personalWpa" => row
                .get("personal_wpa")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            "globalWpa" => row
                .get("global_wpa")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            "damagePerMinute" => row
                .get("damage_per_minute")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            "champWpa" => row
                .get("champions")
                .and_then(serde_json::Value::as_array)
                .and_then(|champs| {
                    champs
                        .iter()
                        .filter_map(|c| c.get("wpa").and_then(serde_json::Value::as_f64))
                        .fold(None, |acc: Option<f64>, v| {
                            Some(acc.map_or(v, |m| m.max(v)))
                        })
                })
                .unwrap_or(-999.0),
            _ => row
                .get("global_wpa")
                .or_else(|| row.get("wpa"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
        };

        if sort_key == "name" {
            rows.sort_by(|left, right| {
                let left_name = left
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let right_name = right
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let ord = left_name
                    .to_ascii_lowercase()
                    .cmp(&right_name.to_ascii_lowercase());
                if descending {
                    ord.reverse()
                } else {
                    ord
                }
            });
        } else {
            rows.sort_by(|left, right| {
                let ord = sort_val(left)
                    .partial_cmp(&sort_val(right))
                    .unwrap_or(std::cmp::Ordering::Equal);
                if descending {
                    ord.reverse()
                } else {
                    ord
                }
            });
        }

        let total = rows.len();
        let paged: Vec<serde_json::Value> = rows.drain(..).skip(offset).take(limit).collect();
        *rows = paged;
        if let Some(obj) = enriched.as_object_mut() {
            obj.insert("total".to_string(), serde_json::Value::from(total));
            obj.insert("offset".to_string(), serde_json::Value::from(offset));
            obj.insert("limit".to_string(), serde_json::Value::from(limit));
        }
    }

    Ok(enriched)
}

#[tauri::command]
pub async fn api_global_aram_statistics(
    supervisor: tauri::State<'_, AgentSupervisor>,
) -> Result<serde_json::Value, String> {
    let response = supervisor.request("global_aram_statistics").await?;
    enrich_global_statistics(response).await
}

#[tauri::command]
pub async fn api_global_aram_config(
    supervisor: tauri::State<'_, AgentSupervisor>,
) -> Result<serde_json::Value, String> {
    supervisor.request("global_aram_config").await
}

#[tauri::command]
pub async fn api_set_global_aram_limit(
    limit: u32,
    supervisor: tauri::State<'_, AgentSupervisor>,
) -> Result<serde_json::Value, String> {
    supervisor
        .request_with_args(
            "set_global_aram_limit",
            serde_json::json!({ "limit": limit }),
        )
        .await
}

#[tauri::command]
pub async fn api_set_global_aram_workers(
    workers: u32,
    supervisor: tauri::State<'_, AgentSupervisor>,
) -> Result<serde_json::Value, String> {
    supervisor
        .request_with_args(
            "set_global_aram_workers",
            serde_json::json!({ "workers": workers }),
        )
        .await
}

#[tauri::command]
pub async fn api_reset_global_aram(
    supervisor: tauri::State<'_, AgentSupervisor>,
) -> Result<serde_json::Value, String> {
    supervisor.request("reset_global_aram").await
}

#[tauri::command]
pub async fn api_pause_global_aram_crawler(
    paused: bool,
    supervisor: tauri::State<'_, AgentSupervisor>,
) -> Result<serde_json::Value, String> {
    supervisor
        .request_with_args(
            "pause_global_aram_crawler",
            serde_json::json!({ "paused": paused }),
        )
        .await
}

#[tauri::command]
pub async fn api_stop_global_aram_crawler(
    supervisor: tauri::State<'_, AgentSupervisor>,
) -> Result<serde_json::Value, String> {
    supervisor.request("stop_global_aram_crawler").await
}

#[tauri::command]
pub fn api_get_logs(limit: Option<usize>) -> Result<Vec<crate::logger::LogEntry>, String> {
    Ok(crate::logger::read_recent_logs(limit.unwrap_or(300)))
}

#[tauri::command]
pub fn api_log(
    level: String,
    source: String,
    message: String,
    context: Option<String>,
) -> Result<(), String> {
    let level = match level.to_ascii_lowercase().as_str() {
        "debug" => Level::Debug,
        "info" => Level::Info,
        "warn" => Level::Warn,
        "error" => Level::Error,
        _ => return Err("invalid log level".to_string()),
    };
    logger::write(level, &source, &message, context.as_deref());
    Ok(())
}
