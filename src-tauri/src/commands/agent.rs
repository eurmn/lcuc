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
    let response = supervisor.request_with_args("statistics", query).await?;
    enrich_statistics_response(response).await
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
