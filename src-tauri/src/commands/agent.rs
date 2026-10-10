use crate::agent_supervisor::{AgentStatus, AgentSupervisor};
use crate::logger::{self, Level};

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
