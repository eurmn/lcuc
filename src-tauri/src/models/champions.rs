//! # Champion Domain Models
//!
//! Strongly typed models for League of Legends champions from CommunityDragon.

use serde::{Deserialize, Serialize};

/// Summary metadata for a champion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionSummary {
    pub id: i64,
    pub name: String,
    pub alias: String,
    pub square_icon_url: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub has_saved_runes: bool,
}

/// Raw item from CDragon `champion-summary.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CDragonChampionItem {
    pub id: i64,
    pub name: String,
    pub alias: String,
    #[serde(rename = "squarePortraitPath", default)]
    pub square_portrait_path: String,
    #[serde(default)]
    pub roles: Vec<String>,
}
