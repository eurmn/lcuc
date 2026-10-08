//! # Coachless.gg Rune Statistics Provider
//!
//! Implementation of [`RuneStatsProvider`] using `https://api.coachless.gg/api`.
//! Handles caching, parallel requests, and CDragon metadata enrichment.

use super::RuneStatsProvider;
use crate::models::{
    BuilderQuery, CoachlessCommonFilters, CoachlessKeystonePayload, CoachlessPatch,
    CoachlessRuneStatItem, CoachlessRunesForTreePayload, CoachlessRunesForTreeResponse,
    CoachlessSecondaryTreeItem, CoachlessSecondaryTreePlaycountPayload, CoachlessShardsPayload,
    CoachlessShardsResponse, EnrichedKeystoneOption, EnrichedRuneOption, RuneBuilderOverview,
    RuneEffectMetric, SecondaryTreeOption, StatShardOption, StatShardsStats, TreePerksStats,
    TreeRowStats,
};
use crate::services::rune_tree::{
    ensure_cdragon_cache, get_perk_details, get_perk_info, get_perk_style_id, get_style_info,
    get_style_perk_order,
};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const COACHLESS_BASE: &str = "https://api.coachless.gg/api";
const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// Coachless telemetry provider client.
#[derive(Clone)]
pub struct CoachlessProvider {
    client: reqwest::Client,
    base_url: String,
    cache: Arc<RwLock<HashMap<String, (Instant, String)>>>,
}

impl Default for CoachlessProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CoachlessProvider {
    /// Creates a new `CoachlessProvider` instance with default headers.
    pub fn new() -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            reqwest::header::ORIGIN,
            reqwest::header::HeaderValue::from_static("https://www.coachless.gg"),
        );
        headers.insert(
            reqwest::header::REFERER,
            reqwest::header::HeaderValue::from_static("https://www.coachless.gg/"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url: COACHLESS_BASE.to_string(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Converts generic [`BuilderQuery`] into Coachless [`CoachlessCommonFilters`].
    fn build_common_filters(&self, query: &BuilderQuery) -> CoachlessCommonFilters {
        let patch = if let Some(p) = &query.patch {
            CoachlessPatch {
                major: p.major,
                patch: p.patch,
                patch_additions: p.patch_additions,
            }
        } else {
            CoachlessPatch::default()
        };

        let tiers = if query.league_tiers.is_empty() {
            vec![4, 5, 6, 7]
        } else {
            query.league_tiers.clone()
        };

        let champ_id = if query.champion_id <= 0 { 64 } else { query.champion_id };

        CoachlessCommonFilters {
            patch,
            champion_ids: vec![champ_id],
            matchup_champion_ids: None,
            league_tiers: tiers,
            regions: None,
            role: query.role,
        }
    }

    /// Internal helper to execute cached POST requests against Coachless.
    async fn post_json_cached<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        payload: &T,
    ) -> Result<R, String> {
        let payload_str = serde_json::to_string(payload)
            .map_err(|e| format!("Failed to serialize payload: {}", e))?;
        let cache_key = format!("{}:{}", endpoint, payload_str);

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some((inserted_at, cached_json)) = cache.get(&cache_key) {
                if inserted_at.elapsed() < CACHE_TTL {
                    if let Ok(res) = serde_json::from_str::<R>(cached_json) {
                        return Ok(res);
                    }
                }
            }
        }

        let url = format!("{}{}", self.base_url, endpoint);
        let resp = self
            .client
            .post(&url)
            .body(payload_str)
            .send()
            .await
            .map_err(|e| format!("Coachless request failed for {}: {}", endpoint, e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Coachless API returned {} on {}: {}", status, endpoint, text));
        }

        let raw_text = resp
            .text()
            .await
            .map_err(|e| format!("Failed reading response text from {}: {}", endpoint, e))?;

        let parsed: R = serde_json::from_str(&raw_text)
            .map_err(|e| format!("Failed parsing JSON response from {}: {}", endpoint, e))?;

        // Save to cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, (Instant::now(), raw_text));
        }

        Ok(parsed)
    }

    /// Helper to convert a raw list of [`CoachlessRuneStatItem`] into [`EnrichedRuneOption`] with row-relative pick rates.
    async fn enrich_rune_items(
        &self,
        items: &[CoachlessRuneStatItem],
        selected_perk_id: Option<i64>,
    ) -> Vec<EnrichedRuneOption> {
        let row_total: i64 = items.iter().map(|i| i.occurrence).sum();
        let mut enriched = Vec::with_capacity(items.len());

        for item in items {
            let (name, icon_url, long_desc) = get_perk_details(item.rune).await;
            let pick_rate = if row_total > 0 {
                (item.occurrence as f64 / row_total as f64) * 100.0
            } else {
                0.0
            };

            let effects = item
                .rune_effects
                .iter()
                .map(|e| RuneEffectMetric {
                    effect_type: e.effect_type,
                    value: e.value.clone(),
                })
                .collect();

            let is_selected = selected_perk_id == Some(item.rune);

            enriched.push(EnrichedRuneOption {
                id: item.rune,
                name,
                icon_url,
                wpa: item.wpa_overall,
                occurrence: item.occurrence,
                pick_rate_pct: (pick_rate * 10.0).round() / 10.0,
                effects,
                selected: is_selected,
                long_desc,
            });
        }

        enriched
    }
}

#[async_trait::async_trait]
impl RuneStatsProvider for CoachlessProvider {
    fn provider_id(&self) -> &'static str {
        "coachless"
    }

    fn display_name(&self) -> &'static str {
        "Coachless.gg"
    }

    async fn get_keystones(
        &self,
        query: &BuilderQuery,
    ) -> Result<Vec<EnrichedKeystoneOption>, String> {
        ensure_cdragon_cache().await?;
        let common_filters = self.build_common_filters(query);
        let payload = CoachlessKeystonePayload {
            keystone: None,
            main_tree: query.primary_style_id.or(Some(8000)),
            tree_to_load: query.primary_style_id.or(Some(8000)),
            common_filters,
        };

        let items: Vec<CoachlessRuneStatItem> = self
            .post_json_cached("/Rune/GetKeystoneData", &payload)
            .await?;

        let total_occurrences: i64 = items.iter().map(|i| i.occurrence).sum();
        let mut enriched_keystones = Vec::with_capacity(items.len());

        for item in &items {
            let (name, icon_url, long_desc) = get_perk_details(item.rune).await;
            let style_id = get_perk_style_id(item.rune).await.unwrap_or(8000);
            let (style_name, style_icon_url) = get_style_info(Some(style_id)).await;

            let pick_rate = if total_occurrences > 0 {
                (item.occurrence as f64 / total_occurrences as f64) * 100.0
            } else {
                0.0
            };

            let effects = item
                .rune_effects
                .iter()
                .map(|e| RuneEffectMetric {
                    effect_type: e.effect_type,
                    value: e.value.clone(),
                })
                .collect();

            let is_selected = query.keystone_id == Some(item.rune);

            enriched_keystones.push(EnrichedKeystoneOption {
                id: item.rune,
                name,
                icon_url,
                style_id,
                style_name,
                style_icon_url,
                wpa: item.wpa_overall,
                occurrence: item.occurrence,
                pick_rate_pct: (pick_rate * 10.0).round() / 10.0,
                effects,
                selected: is_selected,
                long_desc,
            });
        }

        for sid in &[8000, 8100, 8200, 8400, 8300] {
            let (ks_list, _) = get_style_perk_order(*sid).await;
            for kid in ks_list {
                if !enriched_keystones.iter().any(|k| k.id == kid) {
                    let (name, icon_url, long_desc) = get_perk_details(kid).await;
                    let (style_name, style_icon_url) = get_style_info(Some(*sid)).await;
                    enriched_keystones.push(EnrichedKeystoneOption {
                        id: kid,
                        name,
                        icon_url,
                        style_id: *sid,
                        style_name,
                        style_icon_url,
                        wpa: 0.0,
                        occurrence: 0,
                        pick_rate_pct: 0.0,
                        effects: Vec::new(),
                        selected: query.keystone_id == Some(kid),
                        long_desc,
                    });
                }
            }
        }

        // Sort keystones in natural in-game Riot CDragon order
        let mut natural_order = Vec::new();
        for sid in &[8000, 8100, 8200, 8400, 8300] {
            let (ks, _) = get_style_perk_order(*sid).await;
            natural_order.extend(ks);
        }
        enriched_keystones.sort_by_key(|k| {
            natural_order.iter().position(|&id| id == k.id).unwrap_or(999)
        });

        Ok(enriched_keystones)
    }

    async fn get_tree_runes(
        &self,
        main_tree: i64,
        tree_to_load: i64,
        keystone: Option<i64>,
        query: &BuilderQuery,
    ) -> Result<TreePerksStats, String> {
        ensure_cdragon_cache().await?;
        let common_filters = self.build_common_filters(query);
        let payload = CoachlessRunesForTreePayload {
            keystone,
            main_tree,
            tree_to_load,
            common_filters,
        };

        let resp: CoachlessRunesForTreeResponse = self
            .post_json_cached("/Rune/GetRunesForKeystoneAndTree", &payload)
            .await?;

        let (style_name, icon_url) = get_style_info(Some(tree_to_load)).await;
        let (_, natural_rows) = get_style_perk_order(tree_to_load).await;

        let mut row1_options = self.enrich_rune_items(&resp.row_ones, None).await;
        let mut row2_options = self.enrich_rune_items(&resp.row_twos, None).await;
        let mut row3_options = self.enrich_rune_items(&resp.row_threes, None).await;

        if natural_rows.len() >= 3 {
            for perk_id in &natural_rows[0] {
                if !row1_options.iter().any(|o| o.id == *perk_id) {
                    let (name, icon_url, long_desc) = get_perk_details(*perk_id).await;
                    row1_options.push(EnrichedRuneOption {
                        id: *perk_id,
                        name,
                        icon_url,
                        wpa: 0.0,
                        occurrence: 0,
                        pick_rate_pct: 0.0,
                        effects: Vec::new(),
                        selected: false,
                        long_desc,
                    });
                }
            }
            for perk_id in &natural_rows[1] {
                if !row2_options.iter().any(|o| o.id == *perk_id) {
                    let (name, icon_url, long_desc) = get_perk_details(*perk_id).await;
                    row2_options.push(EnrichedRuneOption {
                        id: *perk_id,
                        name,
                        icon_url,
                        wpa: 0.0,
                        occurrence: 0,
                        pick_rate_pct: 0.0,
                        effects: Vec::new(),
                        selected: false,
                        long_desc,
                    });
                }
            }
            for perk_id in &natural_rows[2] {
                if !row3_options.iter().any(|o| o.id == *perk_id) {
                    let (name, icon_url, long_desc) = get_perk_details(*perk_id).await;
                    row3_options.push(EnrichedRuneOption {
                        id: *perk_id,
                        name,
                        icon_url,
                        wpa: 0.0,
                        occurrence: 0,
                        pick_rate_pct: 0.0,
                        effects: Vec::new(),
                        selected: false,
                        long_desc,
                    });
                }
            }

            row1_options.sort_by_key(|o| natural_rows[0].iter().position(|&id| id == o.id).unwrap_or(999));
            row2_options.sort_by_key(|o| natural_rows[1].iter().position(|&id| id == o.id).unwrap_or(999));
            row3_options.sort_by_key(|o| natural_rows[2].iter().position(|&id| id == o.id).unwrap_or(999));
        }

        let rows = vec![
            TreeRowStats {
                row_index: 0,
                label: "Tier 1".to_string(),
                options: row1_options,
            },
            TreeRowStats {
                row_index: 1,
                label: "Tier 2".to_string(),
                options: row2_options,
            },
            TreeRowStats {
                row_index: 2,
                label: "Tier 3".to_string(),
                options: row3_options,
            },
        ];

        Ok(TreePerksStats {
            style_id: tree_to_load,
            style_name,
            icon_url,
            rows,
        })
    }

    async fn get_secondary_tree_playcounts(
        &self,
        tree: i64,
        keystone: Option<i64>,
        query: &BuilderQuery,
    ) -> Result<Vec<SecondaryTreeOption>, String> {
        ensure_cdragon_cache().await?;
        let common_filters = self.build_common_filters(query);
        let payload = CoachlessSecondaryTreePlaycountPayload {
            tree,
            keystone,
            common_filters,
        };

        let items: Vec<CoachlessSecondaryTreeItem> = self
            .post_json_cached("/Rune/GetSecondaryTreePlaycount", &payload)
            .await?;

        let mut max_playcount = 0.0f64;
        for item in &items {
            if item.occurrence > max_playcount {
                max_playcount = item.occurrence;
            }
        }

        let mut candidates = Vec::with_capacity(items.len());
        for item in &items {
            let (style_name, icon_url) = get_style_info(Some(item.tree)).await;
            let is_recommended = (item.occurrence - max_playcount).abs() < f64::EPSILON;

            candidates.push(SecondaryTreeOption {
                style_id: item.tree,
                style_name,
                icon_url,
                playcount_pct: (item.occurrence * 10.0).round() / 10.0,
                is_recommended,
            });
        }

        // Sort by playcount descending
        candidates.sort_by(|a, b| {
            b.playcount_pct
                .partial_cmp(&a.playcount_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(candidates)
    }

    async fn get_shards(
        &self,
        keystone: Option<i64>,
        query: &BuilderQuery,
    ) -> Result<StatShardsStats, String> {
        ensure_cdragon_cache().await?;
        let common_filters = self.build_common_filters(query);
        let payload = CoachlessShardsPayload {
            keystone,
            common_filters,
        };

        let resp: CoachlessShardsResponse = self
            .post_json_cached("/Rune/GetShardsForKeystoneAndTree", &payload)
            .await?;

        async fn enrich_shards(
            items: &[CoachlessRuneStatItem],
            shard_type: &str,
        ) -> Vec<StatShardOption> {
            let row_total: i64 = items.iter().map(|i| i.occurrence).sum();
            let mut options = Vec::with_capacity(items.len());
            for item in items {
                let (name, icon_url, long_desc) = get_perk_details(item.rune).await;
                let pick_rate = if row_total > 0 {
                    (item.occurrence as f64 / row_total as f64) * 100.0
                } else {
                    0.0
                };

                options.push(StatShardOption {
                    id: item.rune,
                    name,
                    icon_url,
                    shard_type: shard_type.to_string(),
                    wpa: item.wpa_overall,
                    occurrence: item.occurrence,
                    pick_rate_pct: (pick_rate * 10.0).round() / 10.0,
                    selected: false,
                    long_desc,
                });
            }
            options
        }

        let offense_order = [5008, 5005, 5007];
        let flex_order = [5008, 5010, 5001];
        let defense_order = [5011, 5013, 5001];

        let mut offense = enrich_shards(&resp.offense, "offense").await;
        let mut flex = enrich_shards(&resp.flex, "flex").await;
        let mut defense = enrich_shards(&resp.defense, "defense").await;

        for sid in &offense_order {
            if !offense.iter().any(|o| o.id == *sid) {
                let (name, icon_url, long_desc) = get_perk_details(*sid).await;
                offense.push(StatShardOption {
                    id: *sid,
                    name,
                    icon_url,
                    shard_type: "offense".to_string(),
                    wpa: 0.0,
                    occurrence: 0,
                    pick_rate_pct: 0.0,
                    selected: false,
                    long_desc,
                });
            }
        }
        for sid in &flex_order {
            if !flex.iter().any(|o| o.id == *sid) {
                let (name, icon_url, long_desc) = get_perk_details(*sid).await;
                flex.push(StatShardOption {
                    id: *sid,
                    name,
                    icon_url,
                    shard_type: "flex".to_string(),
                    wpa: 0.0,
                    occurrence: 0,
                    pick_rate_pct: 0.0,
                    selected: false,
                    long_desc,
                });
            }
        }
        for sid in &defense_order {
            if !defense.iter().any(|o| o.id == *sid) {
                let (name, icon_url, long_desc) = get_perk_details(*sid).await;
                defense.push(StatShardOption {
                    id: *sid,
                    name,
                    icon_url,
                    shard_type: "defense".to_string(),
                    wpa: 0.0,
                    occurrence: 0,
                    pick_rate_pct: 0.0,
                    selected: false,
                    long_desc,
                });
            }
        }

        offense.sort_by_key(|o| offense_order.iter().position(|&id| id == o.id).unwrap_or(999));
        flex.sort_by_key(|o| flex_order.iter().position(|&id| id == o.id).unwrap_or(999));
        defense.sort_by_key(|o| defense_order.iter().position(|&id| id == o.id).unwrap_or(999));

        Ok(StatShardsStats {
            offense,
            flex,
            defense,
        })
    }

    async fn get_builder_overview(
        &self,
        query: &BuilderQuery,
    ) -> Result<RuneBuilderOverview, String> {
        ensure_cdragon_cache().await?;

        // 1. Fetch keystones
        let mut keystones = self.get_keystones(query).await?;

        // 2. Determine selected keystone
        let selected_keystone_id = query
            .keystone_id
            .or_else(|| keystones.first().map(|k| k.id));

        if let Some(selected_id) = selected_keystone_id {
            for k in &mut keystones {
                k.selected = k.id == selected_id;
            }
        }

        // 3. Determine primary style ID
        let primary_style_id = if let Some(selected_id) = selected_keystone_id {
            get_perk_style_id(selected_id).await.unwrap_or(8000)
        } else {
            query.primary_style_id.unwrap_or(8000)
        };

        // 4. Concurrently fetch primary tree, secondary playcounts, and shards
        let primary_tree_fut = self.get_tree_runes(
            primary_style_id,
            primary_style_id,
            selected_keystone_id,
            query,
        );
        let secondary_candidates_fut =
            self.get_secondary_tree_playcounts(primary_style_id, selected_keystone_id, query);
        let shards_fut = self.get_shards(selected_keystone_id, query);

        let (primary_tree_res, secondary_candidates_res, shards_res) =
            tokio::join!(primary_tree_fut, secondary_candidates_fut, shards_fut);

        let primary_tree = primary_tree_res.ok();
        let secondary_candidates = secondary_candidates_res.unwrap_or_default();
        let stat_shards = shards_res.unwrap_or_default();

        // 5. Determine sub style ID
        let sub_style_id = query.sub_style_id.or_else(|| {
            secondary_candidates
                .iter()
                .find(|c| c.style_id != primary_style_id)
                .map(|c| c.style_id)
        });

        // 6. Fetch secondary tree runes if sub_style_id is known
        let secondary_tree = if let Some(sub_id) = sub_style_id {
            self.get_tree_runes(primary_style_id, sub_id, selected_keystone_id, query)
                .await
                .ok()
        } else {
            None
        };

        // 7. Calculate recommended 9-perk setup
        let mut recommended_perk_ids = Vec::new();
        if let Some(ks_id) = selected_keystone_id {
            recommended_perk_ids.push(ks_id);
        }

        // Top perk in each primary row
        if let Some(pt) = &primary_tree {
            for row in &pt.rows {
                if let Some(top_perk) = row.options.iter().max_by_key(|p| p.occurrence) {
                    recommended_perk_ids.push(top_perk.id);
                }
            }
        }

        // Top 2 perks in secondary tree
        if let Some(st) = &secondary_tree {
            let mut all_sec_perks: Vec<&EnrichedRuneOption> =
                st.rows.iter().flat_map(|r| &r.options).collect();
            all_sec_perks.sort_by(|a, b| b.occurrence.cmp(&a.occurrence));
            for perk in all_sec_perks.into_iter().take(2) {
                recommended_perk_ids.push(perk.id);
            }
        }

        // Top stat shard for offense, flex, defense
        if let Some(top_off) = stat_shards.offense.iter().max_by_key(|s| s.occurrence) {
            recommended_perk_ids.push(top_off.id);
        }
        if let Some(top_flex) = stat_shards.flex.iter().max_by_key(|s| s.occurrence) {
            recommended_perk_ids.push(top_flex.id);
        }
        if let Some(top_def) = stat_shards.defense.iter().max_by_key(|s| s.occurrence) {
            recommended_perk_ids.push(top_def.id);
        }

        Ok(RuneBuilderOverview {
            champion_id: query.champion_id,
            role: query.role,
            league_tiers: query.league_tiers.clone(),
            provider: self.display_name().to_string(),
            keystones,
            selected_keystone_id,
            selected_primary_style_id: Some(primary_style_id),
            selected_sub_style_id: sub_style_id,
            primary_tree,
            secondary_tree_candidates: secondary_candidates,
            secondary_tree,
            stat_shards,
            recommended_perk_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coachless_json_parsing() {
        let sample_keystones_json = r#"[
            {"rune":8010,"runeType":0,"wpaOverall":0.33965,"occurrence":324754,"runeEffects":[{"type":3,"value":"296"}]},
            {"rune":8112,"runeType":0,"wpaOverall":-5.2372,"occurrence":19392,"runeEffects":[{"type":0,"value":"1290"}]}
        ]"#;

        let parsed: Vec<CoachlessRuneStatItem> =
            serde_json::from_str(sample_keystones_json).expect("Failed to parse keystones");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].rune, 8010);
        assert_eq!(parsed[0].occurrence, 324754);

        let sample_shards_json = r#"{
            "offense":[{"rune":5005,"runeType":2,"wpaOverall":-0.0394,"occurrence":245998,"runeEffects":[]}],
            "flex":[{"rune":5008,"runeType":3,"wpaOverall":0.0019,"occurrence":254352,"runeEffects":[]}],
            "defense":[{"rune":5001,"runeType":4,"wpaOverall":-0.0998,"occurrence":228824,"runeEffects":[]}]
        }"#;

        let parsed_shards: CoachlessShardsResponse =
            serde_json::from_str(sample_shards_json).expect("Failed to parse shards");
        assert_eq!(parsed_shards.offense[0].rune, 5005);
        assert_eq!(parsed_shards.flex[0].rune, 5008);
        assert_eq!(parsed_shards.defense[0].rune, 5001);
    }
}
