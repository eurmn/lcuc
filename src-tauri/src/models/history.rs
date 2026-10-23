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
