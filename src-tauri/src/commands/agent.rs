use crate::agent_supervisor::{AgentStatus, AgentSupervisor};
use crate::logger::{self, Level};
use crate::services::history::{enrich_global_statistics, enrich_statistics_response};

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

    let mut agent_query = query.clone();
    if let Some(obj) = agent_query.as_object_mut() {
        obj.insert("search".to_string(), serde_json::Value::String(String::new()));
        obj.insert("offset".to_string(), serde_json::Value::from(0));
        obj.insert("limit".to_string(), serde_json::Value::from(10000));
    }

    let response = supervisor.request_with_args("statistics", agent_query).await?;
    let mut enriched = enrich_statistics_response(response).await?;

    if let Some(rows) = enriched.get_mut("rows").and_then(serde_json::Value::as_array_mut) {
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
