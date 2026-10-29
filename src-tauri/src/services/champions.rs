//! # Champions Service
//!
//! Fetches and caches champion metadata from CommunityDragon, resolves champion icons,
//! detects champion associations from rune page names, and tracks saved champion rosters.

use crate::models::{CDragonChampionItem, ChampionSummary};
use crate::services::asset_cache::get_cached_asset_data_uri;
use crate::services::rune_store::RuneStore;
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

const CDRAGON_BASE: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default";
const CHAMPION_SUMMARY_URL: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-summary.json";

lazy_static! {
    static ref CHAMPIONS_CACHE: RwLock<HashMap<i64, ChampionSummary>> = RwLock::new(HashMap::new());
    static ref CHAMPIONS_INITIALIZED: RwLock<bool> = RwLock::new(false);
}

/// Converts an internal game asset path to a public CommunityDragon CDN URL.
fn champion_icon_path_to_url(icon_path: &str) -> String {
    let prefix = "/lol-game-data/assets";
    let normalized = icon_path.trim().to_lowercase();
    if let Some(stripped) = normalized.strip_prefix(prefix) {
        format!("{}{}", CDRAGON_BASE, stripped)
    } else if normalized.starts_with('/') {
        format!("{}{}", CDRAGON_BASE, normalized)
    } else {
        format!("{}/{}", CDRAGON_BASE, normalized)
    }
}

/// Clears the in-memory champion catalog cache so it reloads on the next request.
pub async fn invalidate_champions_cache() {
    let mut initialized = CHAMPIONS_INITIALIZED.write().await;
    *initialized = false;
    let mut cache = CHAMPIONS_CACHE.write().await;
    cache.clear();
}

/// Ensures that the CommunityDragon champion summaries are loaded and cached into memory.
pub async fn ensure_champions_cache() -> Result<(), String> {
    let _ = crate::services::patch_manager::ensure_patch_freshness(None).await;
    {
        let initialized = CHAMPIONS_INITIALIZED.read().await;
        if *initialized {
            return Ok(());
        }
    }

    let cache_file = crate::services::asset_cache::get_cache_dir().join("champion-summary.json");
    let client = reqwest::Client::new();
    let fetch_res = client.get(CHAMPION_SUMMARY_URL).send().await;

    let mut parsed_items: Option<Vec<CDragonChampionItem>> = None;
    if let Ok(res) = fetch_res {
        if res.status().is_success() {
            if let Ok(body) = res.text().await {
                if let Ok(items) = serde_json::from_str::<Vec<CDragonChampionItem>>(&body) {
                    if let Some(parent) = cache_file.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = std::fs::write(&cache_file, &body) {
                        eprintln!("Failed caching champion-summary.json: {e}");
                    }
                    parsed_items = Some(items);
                }
            }
        }
    }

    let resp: Vec<CDragonChampionItem> = match parsed_items {
        Some(items) => items,
        None => {
            if cache_file.exists() {
                let body = std::fs::read_to_string(&cache_file)
                    .map_err(|e| format!("Failed reading disk champion-summary.json: {e}"))?;
                serde_json::from_str(&body)
                    .map_err(|e| format!("Failed parsing disk champion-summary.json: {e}"))?
            } else {
                return Err(
                    "Failed fetching valid champion-summary.json and no disk cache available"
                        .to_string(),
                );
            }
        }
    };

    let mut cache = CHAMPIONS_CACHE.write().await;
    for item in resp {
        if item.id <= 0 || item.id.to_string().starts_with("600") {
            continue; // Skip placeholder/none IDs (-1) and classic champions (600xxx)
        }

        let raw_icon_url = if !item.square_portrait_path.is_empty() {
            champion_icon_path_to_url(&item.square_portrait_path)
        } else {
            format!("{}/v1/champion-icons/{}.png", CDRAGON_BASE, item.id)
        };

        let square_icon_url = get_cached_asset_data_uri(&raw_icon_url).await;

        cache.insert(
            item.id,
            ChampionSummary {
                id: item.id,
                name: item.name,
                alias: item.alias,
                square_icon_url,
                roles: item.roles,
                has_saved_runes: false,
                is_favorite: false,
            },
        );
    }

    let mut initialized = CHAMPIONS_INITIALIZED.write().await;
    *initialized = true;
    Ok(())
}

/// Returns a set of champion IDs that have at least one saved runeset in SQLite.
pub fn get_saved_champion_ids() -> HashSet<i64> {
    if let Ok(store) = RuneStore::new() {
        if let Ok(saved) = store.list_runesets(200) {
            return saved
                .into_iter()
                .filter(|r| r.champion_id > 0)
                .map(|r| r.champion_id)
                .collect();
        }
    }
    HashSet::new()
}

/// Returns a set of champion IDs that are marked as favorites in SQLite.
pub fn get_favorite_champion_ids() -> HashSet<i64> {
    if let Ok(store) = RuneStore::new() {
        if let Ok(favs) = store.get_favorite_champion_ids() {
            return favs;
        }
    }
    HashSet::new()
}

/// Returns all champions sorted alphabetically, with `has_saved_runes` and `is_favorite` populated.
pub async fn get_all_champions() -> Result<Vec<ChampionSummary>, String> {
    ensure_champions_cache().await?;
    let saved_ids = get_saved_champion_ids();
    let favorite_ids = get_favorite_champion_ids();

    let cache = CHAMPIONS_CACHE.read().await;
    let mut list: Vec<ChampionSummary> = cache
        .values()
        .map(|c| {
            let mut item = c.clone();
            item.has_saved_runes = saved_ids.contains(&c.id);
            item.is_favorite = favorite_ids.contains(&c.id);
            item
        })
        .collect();

    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(list)
}

/// Returns the champion roster of champions that currently have runes saved (or fallback to popular champions).
pub async fn get_saved_champion_roster() -> Result<Vec<ChampionSummary>, String> {
    ensure_champions_cache().await?;
    let saved_ids = get_saved_champion_ids();
    let favorite_ids = get_favorite_champion_ids();

    let cache = CHAMPIONS_CACHE.read().await;
    let mut roster: Vec<ChampionSummary> = cache
        .values()
        .filter(|c| saved_ids.contains(&c.id))
        .map(|c| {
            let mut item = c.clone();
            item.has_saved_runes = true;
            item.is_favorite = favorite_ids.contains(&c.id);
            item
        })
        .collect();

    roster.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(roster)
}

/// Detects if a champion name is present in a given string (e.g. rune page name "Aatrox Conqueror").
pub async fn detect_champion_from_name(name: &str) -> Option<ChampionSummary> {
    ensure_champions_cache().await.ok()?;
    let normalized = name.to_lowercase();
    let saved_ids = get_saved_champion_ids();
    let favorite_ids = get_favorite_champion_ids();

    let cache = CHAMPIONS_CACHE.read().await;
    let mut candidates: Vec<&ChampionSummary> = cache.values().collect();
    // Prioritize longer champion names to avoid false positive prefix matches (e.g. "Vi" in "Viktor")
    candidates.sort_by(|a, b| b.name.len().cmp(&a.name.len()));

    for champ in candidates {
        let champ_name_lower = champ.name.to_lowercase();
        let champ_alias_lower = champ.alias.to_lowercase();

        // Check whole-word or substring match
        if normalized.contains(&champ_name_lower) || normalized.contains(&champ_alias_lower) {
            let mut matched = champ.clone();
            matched.has_saved_runes = saved_ids.contains(&champ.id);
            matched.is_favorite = favorite_ids.contains(&champ.id);
            return Some(matched);
        }
    }

    None
}
