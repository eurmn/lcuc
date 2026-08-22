//! # Rune Builder Tauri Commands
//!
//! Exposes modular rune telemetry and build creation endpoints to the Svelte UI
//! via typed JSON IPC commands.

use crate::models::{
    BuilderQuery, EnrichedKeystoneOption, RuneBuilderOverview, SecondaryTreeOption,
    StatShardsStats, TreePerksStats,
};
use crate::services::rune_stats::get_stats_provider;

/// Returns the comprehensive builder overview (keystones, primary tree, secondary trees, stat shards, recommended build)
/// for a given champion, role, and rank tier selection.
#[tauri::command]
pub async fn api_rune_builder_get_overview(
    query: BuilderQuery,
) -> Result<RuneBuilderOverview, String> {
    let provider = get_stats_provider(query.provider.as_deref());
    provider.get_builder_overview(&query).await
}

/// Returns all viable keystones with WPA, occurrence counts, and pick rates.
#[tauri::command]
pub async fn api_rune_builder_get_keystones(
    query: BuilderQuery,
) -> Result<Vec<EnrichedKeystoneOption>, String> {
    let provider = get_stats_provider(query.provider.as_deref());
    provider.get_keystones(&query).await
}

/// Returns the 3-row tier perks for a specific style tree.
#[tauri::command]
pub async fn api_rune_builder_get_tree_runes(
    main_tree: i64,
    tree_to_load: i64,
    keystone: Option<i64>,
    query: BuilderQuery,
) -> Result<TreePerksStats, String> {
    let provider = get_stats_provider(query.provider.as_deref());
    provider
        .get_tree_runes(main_tree, tree_to_load, keystone, &query)
        .await
}

/// Returns the distribution and relative pick popularity of candidate secondary trees.
#[tauri::command]
pub async fn api_rune_builder_get_secondary_playcounts(
    tree: i64,
    keystone: Option<i64>,
    query: BuilderQuery,
) -> Result<Vec<SecondaryTreeOption>, String> {
    let provider = get_stats_provider(query.provider.as_deref());
    provider
        .get_secondary_tree_playcounts(tree, keystone, &query)
        .await
}

/// Returns statistics for the 3 stat shard rows (Offense, Flex, Defense).
#[tauri::command]
pub async fn api_rune_builder_get_shards(
    keystone: Option<i64>,
    query: BuilderQuery,
) -> Result<StatShardsStats, String> {
    let provider = get_stats_provider(query.provider.as_deref());
    provider.get_shards(keystone, &query).await
}
