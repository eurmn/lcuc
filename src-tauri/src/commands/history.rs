use crate::models::MatchHistoryItem;
use crate::services::history::get_current_summoner_match_history;

#[tauri::command]
pub async fn api_match_history(beg_index: Option<u32>, end_index: Option<u32>) -> Result<Vec<MatchHistoryItem>, String> {
    let beg = beg_index.unwrap_or(0);
    let end = end_index.unwrap_or(20);
    get_current_summoner_match_history(beg, end).await
}