use crate::models::ChampionSummary;
use crate::services::champions::{
    detect_champion_from_name, get_all_champions, get_saved_champion_roster,
};
use crate::services::rune_store::RuneStore;

/// Returns all champions sorted alphabetically.
#[tauri::command]
pub async fn api_get_champions() -> Result<Vec<ChampionSummary>, String> {
    get_all_champions().await
}

/// Returns the champion roster of champions that currently have custom runes saved.
#[tauri::command]
pub async fn api_get_champion_roster() -> Result<Vec<ChampionSummary>, String> {
    get_saved_champion_roster().await
}

/// Detects the first champion name referenced inside a rune page name string.
#[tauri::command]
pub async fn api_detect_champion_from_name(
    name: String,
) -> Result<Option<ChampionSummary>, String> {
    Ok(detect_champion_from_name(&name).await)
}

/// Toggles whether a champion is marked as favorite in SQLite.
#[tauri::command]
pub async fn api_toggle_favorite_champion(champion_id: i64) -> Result<bool, String> {
    if champion_id <= 0 {
        return Err("Invalid champion ID".to_string());
    }
    let store = RuneStore::new()?;
    store.toggle_favorite_champion(champion_id)
}

/// Retrieves the last selected champion ID from SQLite.
#[tauri::command]
pub async fn api_get_selected_champion_id() -> Result<Option<i64>, String> {
    let store = RuneStore::new()?;
    store.get_selected_champion_id()
}

/// Persists the last selected champion ID in SQLite.
#[tauri::command]
pub async fn api_set_selected_champion_id(champion_id: Option<i64>) -> Result<(), String> {
    let store = RuneStore::new()?;
    store.set_selected_champion_id(champion_id)
}
