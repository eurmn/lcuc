//! # Generic Rune Builder Domain Models
//!
//! Provider-agnostic models representing enriched rune options, tier matrices,
//! secondary tree distribution, stat shards, and aggregated builder overview payloads.

use serde::{Deserialize, Serialize};

/// League rank tier identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeagueTier {
    Iron = 0,
    Bronze = 1,
    Silver = 2,
    Gold = 3,
    Platinum = 4,
    Emerald = 5,
    Diamond = 6,
    MasterPlus = 7,
}

/// League role / lane identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeagueRole {
    Top = 0,
    Jungle = 1,
    Mid = 2,
    Adc = 3,
    Support = 4,
    AllRoles = 5,
}

/// Game patch filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchFilter {
    pub major: u32,
    pub patch: u32,
    pub patch_additions: u32,
}

impl Default for PatchFilter {
    fn default() -> Self {
        Self {
            major: 16,
            patch: 16,
            patch_additions: 0,
        }
    }
}

/// Query parameters for fetching rune statistics from any provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderQuery {
    pub champion_id: i64,
    #[serde(default = "default_role")]
    pub role: u32,
    #[serde(default = "default_tiers")]
    pub league_tiers: Vec<u32>,
    #[serde(default)]
    pub patch: Option<PatchFilter>,
    #[serde(default)]
    pub keystone_id: Option<i64>,
    #[serde(default)]
    pub primary_style_id: Option<i64>,
    #[serde(default)]
    pub sub_style_id: Option<i64>,
    #[serde(default)]
    pub provider: Option<String>,
}

fn default_role() -> u32 {
    1 // Jungle default (e.g. for Lee Sin)
}

fn default_tiers() -> Vec<u32> {
    vec![4, 5, 6, 7] // Platinum, Emerald, Diamond, Master+
}

/// In-game rune telemetry metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneEffectMetric {
    pub effect_type: i32,
    pub value: String,
}

/// Enriched keystone option with CDragon assets and statistical performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedKeystoneOption {
    pub id: i64,
    pub name: String,
    pub icon_url: String,
    pub style_id: i64,
    pub style_name: String,
    pub style_icon_url: String,
    pub wpa: f64,
    pub occurrence: i64,
    pub pick_rate_pct: f64,
    pub effects: Vec<RuneEffectMetric>,
    pub selected: bool,
}

/// Enriched standard perk option with performance statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedRuneOption {
    pub id: i64,
    pub name: String,
    pub icon_url: String,
    pub wpa: f64,
    pub occurrence: i64,
    pub pick_rate_pct: f64,
    pub effects: Vec<RuneEffectMetric>,
    pub selected: bool,
}

/// A row of tier perks with statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeRowStats {
    pub row_index: usize,
    pub label: String,
    pub options: Vec<EnrichedRuneOption>,
}

/// Structured perk tree containing 3 rows of stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreePerksStats {
    pub style_id: i64,
    pub style_name: String,
    pub icon_url: String,
    pub rows: Vec<TreeRowStats>,
}

/// Candidate secondary style tree with relative pick popularity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondaryTreeOption {
    pub style_id: i64,
    pub style_name: String,
    pub icon_url: String,
    pub playcount_pct: f64,
    pub is_recommended: bool,
}

/// Enriched stat shard option with performance stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatShardOption {
    pub id: i64,
    pub name: String,
    pub icon_url: String,
    pub shard_type: String, // "offense", "flex", "defense"
    pub wpa: f64,
    pub occurrence: i64,
    pub pick_rate_pct: f64,
    pub selected: bool,
}

/// 3 rows of stat shards (Offense, Flex, Defense).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatShardsStats {
    pub offense: Vec<StatShardOption>,
    pub flex: Vec<StatShardOption>,
    pub defense: Vec<StatShardOption>,
}

/// Comprehensive aggregated overview for the Rune Builder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneBuilderOverview {
    pub champion_id: i64,
    pub role: u32,
    pub league_tiers: Vec<u32>,
    pub provider: String,
    pub keystones: Vec<EnrichedKeystoneOption>,
    pub selected_keystone_id: Option<i64>,
    pub selected_primary_style_id: Option<i64>,
    pub selected_sub_style_id: Option<i64>,
    pub primary_tree: Option<TreePerksStats>,
    pub secondary_tree_candidates: Vec<SecondaryTreeOption>,
    pub secondary_tree: Option<TreePerksStats>,
    pub stat_shards: StatShardsStats,
    pub recommended_perk_ids: Vec<i64>,
}
