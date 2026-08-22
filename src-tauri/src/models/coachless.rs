//! # Coachless.gg Raw API DTO Models
//!
//! Strongly typed serde models matching the exact payloads and responses of `https://api.coachless.gg/api`.

use serde::{Deserialize, Serialize};

/// Patch version specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachlessPatch {
    pub major: u32,
    pub patch: u32,
    #[serde(rename = "patchAdditions")]
    pub patch_additions: u32,
}

impl Default for CoachlessPatch {
    fn default() -> Self {
        Self {
            major: 16,
            patch: 16,
            patch_additions: 0,
        }
    }
}

/// Common filters block sent in all Coachless.gg POST endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachlessCommonFilters {
    pub patch: CoachlessPatch,
    #[serde(rename = "championIds")]
    pub champion_ids: Vec<i64>,
    #[serde(rename = "matchupChampionIds")]
    pub matchup_champion_ids: Option<Vec<i64>>,
    #[serde(rename = "leagueTiers")]
    pub league_tiers: Vec<u32>,
    pub regions: Option<Vec<String>>,
    pub role: u32,
}

/// Request payload for `POST /Rune/GetKeystoneData`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachlessKeystonePayload {
    pub keystone: Option<i64>,
    #[serde(rename = "mainTree")]
    pub main_tree: Option<i64>,
    #[serde(rename = "treeToLoad")]
    pub tree_to_load: Option<i64>,
    #[serde(rename = "commonFilters")]
    pub common_filters: CoachlessCommonFilters,
}

/// In-game rune telemetry metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachlessRuneEffect {
    #[serde(rename = "type")]
    pub effect_type: i32,
    pub value: String,
}

/// Single rune telemetry entry returned in keystone/tree responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachlessRuneStatItem {
    pub rune: i64,
    #[serde(rename = "runeType")]
    pub rune_type: i32,
    #[serde(rename = "wpaOverall")]
    pub wpa_overall: f64,
    pub occurrence: i64,
    #[serde(rename = "runeEffects", default)]
    pub rune_effects: Vec<CoachlessRuneEffect>,
}

/// Request payload for `POST /Rune/GetRunesForKeystoneAndTree`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachlessRunesForTreePayload {
    pub keystone: Option<i64>,
    #[serde(rename = "mainTree")]
    pub main_tree: i64,
    #[serde(rename = "treeToLoad")]
    pub tree_to_load: i64,
    #[serde(rename = "commonFilters")]
    pub common_filters: CoachlessCommonFilters,
}

/// Response from `POST /Rune/GetRunesForKeystoneAndTree`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoachlessRunesForTreeResponse {
    #[serde(rename = "rowOnes", default)]
    pub row_ones: Vec<CoachlessRuneStatItem>,
    #[serde(rename = "rowTwos", default)]
    pub row_twos: Vec<CoachlessRuneStatItem>,
    #[serde(rename = "rowThrees", default)]
    pub row_threes: Vec<CoachlessRuneStatItem>,
}

/// Request payload for `POST /Rune/GetSecondaryTreePlaycount`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachlessSecondaryTreePlaycountPayload {
    pub tree: i64,
    pub keystone: Option<i64>,
    #[serde(rename = "commonFilters")]
    pub common_filters: CoachlessCommonFilters,
}

/// Single item in `POST /Rune/GetSecondaryTreePlaycount` response array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachlessSecondaryTreeItem {
    pub tree: i64,
    pub occurrence: f64,
}

/// Request payload for `POST /Rune/GetShardsForKeystoneAndTree`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachlessShardsPayload {
    pub keystone: Option<i64>,
    #[serde(rename = "commonFilters")]
    pub common_filters: CoachlessCommonFilters,
}

/// Response from `POST /Rune/GetShardsForKeystoneAndTree`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoachlessShardsResponse {
    #[serde(default)]
    pub offense: Vec<CoachlessRuneStatItem>,
    #[serde(default)]
    pub flex: Vec<CoachlessRuneStatItem>,
    #[serde(default)]
    pub defense: Vec<CoachlessRuneStatItem>,
}
