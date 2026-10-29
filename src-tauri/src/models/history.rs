use super::ChampionSummary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchHistoryItemInfo {
    pub id: i32,
    pub name: String,
    pub icon_url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub price_total: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchHistoryAugmentInfo {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    #[serde(default)]
    pub rarity: Option<String>,
    #[serde(default)]
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchHistoryItem {
    pub game_id: u64,
    pub champion: Option<ChampionSummary>,
    pub win: bool,
    pub game_mode: String,
    pub game_duration_secs: u32,
    pub game_creation_date: String,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub champ_level: i64,
    pub minions_killed: i64,
    pub gold_earned: i64,
    pub spell1_id: u16,
    pub spell2_id: u16,
    pub items: Vec<Option<MatchHistoryItemInfo>>,
    #[serde(default)]
    pub damage_dealt_to_champions: Option<i64>,
    #[serde(default)]
    pub augments: Vec<MatchHistoryAugmentInfo>,
    pub map_id: u16,
    #[serde(default)]
    pub map_name: Option<String>,
    #[serde(default)]
    pub patch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemBuildItem {
    pub id: i64,
    pub name: String,
    pub icon_url: String,
    pub price_total: i64,
    #[serde(default)]
    pub item_class: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub win_rate: f64,
    #[serde(default)]
    pub wpa: f64,
    #[serde(default)]
    pub games: usize,
    #[serde(default)]
    pub pick_rate: f64,
    #[serde(default)]
    pub situational_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemStageItem {
    pub stage: u32,
    pub title: String,
    pub timing: String,
    pub items: Vec<MayhemBuildItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemLiveAdvice {
    pub current_stage: u32,
    pub stage_title: String,
    pub immediate_purchases: Vec<MayhemBuildItem>,
    pub next_target: Option<MayhemBuildItem>,
    pub gold_needed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemBuildSpell {
    pub id: u64,
    pub name: String,
    pub icon_url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub win_rate: f64,
    #[serde(default)]
    pub wpa: f64,
    #[serde(default)]
    pub games: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemBuildAugment {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    #[serde(default)]
    pub rarity: Option<String>,
    #[serde(default)]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub win_rate: f64,
    #[serde(default)]
    pub wpa: f64,
    #[serde(default)]
    pub games: usize,
    #[serde(default)]
    pub pick_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemBuildStrategy {
    pub id: String,
    pub label: String,
    pub metric_name: String,
    pub avg_metric_value: String,
    pub raw_metric_value: f64,
    pub build_win_rate: f64,
    pub build_wpa: f64,
    #[serde(default)]
    pub starter_item: Option<MayhemBuildItem>,
    pub items: Vec<MayhemBuildItem>,
    #[serde(default)]
    pub alternative_items: Vec<MayhemBuildItem>,
    #[serde(default)]
    pub situational_items: Vec<MayhemBuildItem>,
    #[serde(default)]
    pub stages: Vec<MayhemStageItem>,
    pub spells: Vec<MayhemBuildSpell>,
    #[serde(default)]
    pub all_spells: Vec<MayhemBuildSpell>,
    pub augments: Vec<MayhemBuildAugment>,
    pub sample_games: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemChampionBuildSummary {
    pub champion_id: i64,
    pub champion_name: String,
    pub icon_url: String,
    pub champion_win_rate: f64,
    pub champion_wpa: f64,
    pub total_games: usize,
    pub strategies: Vec<MayhemBuildStrategy>,
    #[serde(default)]
    pub applied_augment_ids: Vec<String>,
    #[serde(default)]
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemAugmentItem {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    #[serde(default)]
    pub rarity: Option<String>,
    #[serde(default)]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub games: usize,
    #[serde(default)]
    pub wins: usize,
    #[serde(default)]
    pub win_rate: f64,
    #[serde(default)]
    pub wpa: f64,
    #[serde(default)]
    pub pick_rate: f64,
    #[serde(default)]
    pub global_games: usize,
    #[serde(default)]
    pub global_win_rate: f64,
    #[serde(default)]
    pub global_wpa: f64,
    #[serde(default)]
    pub global_pick_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MayhemAugmentsQuery {
    pub champion_id: Option<i64>,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub sort_key: Option<String>,
    #[serde(default)]
    pub descending: Option<bool>,
    #[serde(default)]
    pub page: Option<usize>,
    #[serde(default)]
    pub page_size: Option<usize>,
    #[serde(default)]
    pub only_sampled: Option<bool>,
    #[serde(default)]
    pub hide_low_pickrate: Option<bool>,
    #[serde(default)]
    pub with_augment_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemAugmentsResponse {
    pub total_items: usize,
    pub total_pages: usize,
    pub page: usize,
    pub page_size: usize,
    pub champion_id: Option<i64>,
    pub champion_name: Option<String>,
    pub champion_win_rate: f64,
    #[serde(default)]
    pub total_games: usize,
    pub items: Vec<MayhemAugmentItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMayhemBuild {
    pub id: String,
    pub champion_id: i64,
    pub champion_name: String,
    pub strategy_label: String,
    pub augment_ids: Vec<String>,
    pub build: MayhemBuildStrategy,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMayhemBuildPayload {
    pub champion_id: i64,
    pub champion_name: String,
    pub strategy_label: String,
    pub augment_ids: Vec<String>,
    pub build: MayhemBuildStrategy,
}
