use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildEngineRequest {
    pub id: u64,
    pub champion_id: u32,
    pub owned_items: Vec<u32>,
    pub owned_augments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildEngineResponse {
    pub id: u64,
    pub result: Option<BuildRecommendationResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveComboSynergy {
    pub combo_type: String, // "item-augment" or "item-item"
    pub item_ids: Vec<u32>,
    pub augment_ids: Vec<String>,
    pub label: String,
    pub win_rate: f64,
    pub wpa: f64,
    pub sample_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemRecommendation {
    pub item_id: u32,
    pub win_rate: f64,
    pub lift: f64,
    pub sample_size: usize,
    pub is_boot: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EnginePerformanceStats {
    pub load_time_ms: f64,
    pub compute_time_ms: f64,
    pub total_time_ms: f64,
    pub match_count: usize,
    pub cache_hit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildRecommendationResult {
    pub champion_id: u32,
    pub active_combos: Vec<ActiveComboSynergy>,
    pub next_items: Vec<ItemRecommendation>,
    pub recommended_boots: Option<ItemRecommendation>,
    pub completed_build: Vec<ItemRecommendation>,
    pub alternative_items: Vec<ItemRecommendation>,
    pub stats: EnginePerformanceStats,
}
