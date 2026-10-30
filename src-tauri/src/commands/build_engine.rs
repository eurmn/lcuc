use crate::build_engine::host::request_build_recommendation;
use crate::build_engine::models::BuildRecommendationResult;

#[tauri::command]
pub async fn api_compute_mayhem_build_recommendation(
    champion_id: u32,
    owned_items: Vec<u32>,
    owned_augments: Vec<String>,
) -> Result<BuildRecommendationResult, String> {
    request_build_recommendation(champion_id, owned_items, owned_augments).await
}
