//! # Modular Rune Statistics Provider Abstraction
//!
//! Provides a decoupled trait interface for querying champion rune performance metrics
//! across different telemetry providers (Coachless.gg, and future providers like Lolalytics, OP.GG, etc.).

pub mod coachless;

use crate::models::{
    BuilderQuery, EnrichedKeystoneOption, RuneBuilderOverview, SecondaryTreeOption,
    StatShardsStats, TreePerksStats,
};
use coachless::CoachlessProvider;
use std::sync::Arc;

/// Trait defining the standard interface for any rune telemetry and statistics provider.
#[async_trait::async_trait]
pub trait RuneStatsProvider: Send + Sync {
    /// Identifier of the provider (e.g. `"coachless"`).
    fn provider_id(&self) -> &'static str;

    /// Display name of the provider (e.g. `"Coachless.gg"`).
    fn display_name(&self) -> &'static str;

    /// Fetches all viable keystones with WPA, occurrence counts, and pick rates.
    async fn get_keystones(
        &self,
        query: &BuilderQuery,
    ) -> Result<Vec<EnrichedKeystoneOption>, String>;

    /// Fetches the 3-row tier perks for a specific style tree.
    async fn get_tree_runes(
        &self,
        main_tree: i64,
        tree_to_load: i64,
        keystone: Option<i64>,
        query: &BuilderQuery,
    ) -> Result<TreePerksStats, String>;

    /// Fetches the popularity distribution of candidate secondary trees.
    async fn get_secondary_tree_playcounts(
        &self,
        tree: i64,
        keystone: Option<i64>,
        query: &BuilderQuery,
    ) -> Result<Vec<SecondaryTreeOption>, String>;

    /// Fetches performance stats for Offense, Flex, and Defense stat shards.
    async fn get_shards(
        &self,
        keystone: Option<i64>,
        query: &BuilderQuery,
    ) -> Result<StatShardsStats, String>;

    /// Fetches a unified, aggregated overview for the complete rune builder.
    async fn get_builder_overview(
        &self,
        query: &BuilderQuery,
    ) -> Result<RuneBuilderOverview, String>;
}

/// Returns the designated statistics provider instance.
pub fn get_stats_provider(name: Option<&str>) -> Arc<dyn RuneStatsProvider> {
    match name.unwrap_or("coachless").to_lowercase().as_str() {
        "coachless" => Arc::new(CoachlessProvider::new()),
        // Future providers can be registered here:
        // "lolalytics" => Arc::new(LolalyticsProvider::new()),
        _ => Arc::new(CoachlessProvider::new()),
    }
}
