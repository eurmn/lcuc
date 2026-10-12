#[tauri::command]
pub async fn api_delete_global_aram_db() -> Result<(), String> {
    use std::fs;
    use std::path::PathBuf;
    let path = dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu-companion")
        .join("global-aram.db");
    fs::remove_file(&path).map_err(|e| format!("failed to delete global-aram.db: {e}"))?;
    Ok(())
}
