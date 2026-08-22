//! # Champions Tauri Commands
//!
//! Exposes champion listing, search, roster retrieval, and name-based detection endpoints to the frontend.

use crate::models::ChampionSummary;
use crate::services::champions::{
    detect_champion_from_name, get_all_champions, get_saved_champion_roster,
};

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
