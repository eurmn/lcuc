use crate::services::friends::{remove_friends, FriendRemovalResult};

#[tauri::command]
pub async fn api_remove_friends() -> Result<FriendRemovalResult, String> {
    remove_friends().await
}
