use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneSetRecord {
    pub id: i64,
    pub summoner_puuid: Option<String>,
    pub champion_id: i64,
    pub game_id: Option<i64>,
    pub rune_page_id: Option<i64>,
    pub rune_page_name: String,
    pub primary_style_id: Option<i64>,
    pub sub_style_id: Option<i64>,
    pub selected_perk_ids: Vec<i64>,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneSetSummary {
    pub id: i64,
    pub rune_page_name: String,
    pub primary_style_id: Option<i64>,
    pub sub_style_id: Option<i64>,
    pub champion_id: i64,
    pub source: String,
    pub created_at: String,
}
