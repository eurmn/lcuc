use crate::logger;
use crate::models::{
    ChampionSummary, MatchHistoryAugmentInfo, MatchHistoryItem, MatchHistoryItemInfo,
    MayhemAugmentItem, MayhemAugmentsQuery, MayhemAugmentsResponse, MayhemBuildAugment,
    MayhemBuildItem, MayhemBuildSpell, MayhemBuildStrategy, MayhemChampionBuildSummary,
    MayhemLiveAdvice, MayhemStageItem, SaveMayhemBuildPayload, SavedMayhemBuild,
};
use crate::services::asset_cache::{get_cache_dir, get_cached_asset_data_uri};
use crate::services::champions::get_all_champions;
use crate::services::lcu_client::LcuClient;
use futures_util::future::join_all;
use lazy_static::lazy_static;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

const CDRAGON_ITEMS_URL: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/items.json";
const CDRAGON_ITEM_ICON_BASE: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d";
const CDRAGON_GAME_ITEM_ICON_BASE: &str =
    "https://raw.communitydragon.org/latest/game/assets/items/icons2d";
const CDRAGON_CHERRY_AUGMENTS_URL: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/cherry-augments.json";
const CDRAGON_KIWI_AUGMENTS_URL: &str =
    "https://raw.communitydragon.org/latest/game/maps/modespecificdata/kiwi.bin.json";
const CDRAGON_STRINGTABLE_URL: &str =
    "https://raw.communitydragon.org/latest/game/en_us/data/menu/en_us/lol.stringtable.json";
const CDRAGON_MAPS_URL: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/maps.json";
const CDRAGON_SUMMONER_SPELLS_URL: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/summoner-spells.json";

fn history_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu-companion")
}

fn route_log_dir(route: &str) -> PathBuf {
    history_data_dir()
        .join("logs")
        .join(route.trim_start_matches('/'))
}

fn request_date() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "unknown-time".to_string())
}

fn log_history_response(route: &str, game_id: Option<u64>, response: &Value) {
    let directory = match game_id {
        Some(game_id) => route_log_dir(route).join(format!("{game_id}.json")),
        None => route_log_dir(route),
    };
    if let Err(error) = fs::create_dir_all(&directory) {
        logger::error(
            "tauri.history",
            &format!("Failed creating response log directory: {error}"),
        );
        return;
    }
    let file_name = match game_id {
        Some(_) => format!("{}.json", request_date()),
        None => format!("history_{}.json", request_date()),
    };
    let path = directory.join(file_name);
    if let Err(error) = serde_json::to_vec_pretty(response)
        .map_err(|error| error.to_string())
        .and_then(|bytes| fs::write(path, bytes).map_err(|error| error.to_string()))
    {
        logger::error(
            "tauri.history",
            &format!("Failed writing response log: {error}"),
        );
    }
}

fn detail_cache_path(game_id: u64) -> PathBuf {
    history_data_dir()
        .join("cache")
        .join("match-history")
        .join(format!("{game_id}.json"))
}

pub(crate) fn read_detail_cache(game_id: u64) -> Option<Value> {
    let path = detail_cache_path(game_id);
    if let Some(detail) = fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    {
        return Some(detail);
    }

    // Secondary check: global-aram.db aram_games table
    let db_path = crate::agent::global_aram_database_path();
    if db_path.exists() {
        if let Ok(conn) = rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            let row_res: Result<String, _> = conn.query_row(
                "SELECT detail_json FROM aram_games WHERE game_id = ?1 LIMIT 1",
                rusqlite::params![game_id as i64],
                |row| row.get(0),
            );
            if let Ok(detail_json) = row_res {
                if let Ok(parsed) = serde_json::from_str::<Value>(&detail_json) {
                    write_detail_cache(game_id, &parsed);
                    return Some(parsed);
                }
            }
        }
    }

    None
}

pub(crate) fn write_detail_cache(game_id: u64, response: &Value) {
    let path = detail_cache_path(game_id);
    if let Some(directory) = path.parent() {
        if let Err(error) = fs::create_dir_all(directory) {
            logger::error(
                "tauri.history",
                &format!("Failed creating detail cache directory: {error}"),
            );
            return;
        }
    }
    if let Err(error) = serde_json::to_vec(response)
        .map_err(|error| error.to_string())
        .and_then(|bytes| fs::write(path, bytes).map_err(|error| error.to_string()))
    {
        logger::error(
            "tauri.history",
            &format!("Failed writing detail cache: {error}"),
        );
    }
}

fn augment_resolution_cache_path() -> PathBuf {
    history_data_dir()
        .join("cache")
        .join("match-history")
        .join("augments_v9.json")
}

fn augment_catalog_cache_path() -> PathBuf {
    history_data_dir()
        .join("cache")
        .join("match-history")
        .join("augment_catalog_v10.json")
}

/// Clears all in-memory and on-disk item, map, and augment caches (triggered on patch updates).
pub async fn invalidate_history_caches() {
    *ITEMS_CACHE.write().await = None;
    *MAPS_CACHE.write().await = None;
    *AUGMENTS_CATALOG_CACHE.write().await = None;
    *RESOLVED_AUGMENTS_CACHE.write().await = None;
    *SPELLS_CACHE.write().await = None;
    invalidate_mayhem_builds_cache();
    let _ = fs::remove_file(augment_resolution_cache_path());
    let _ = fs::remove_file(augment_catalog_cache_path());
}

fn read_augment_resolution_cache() -> HashMap<String, MatchHistoryAugmentInfo> {
    let path = augment_resolution_cache_path();
    let raw: HashMap<String, MatchHistoryAugmentInfo> = fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    // Filter out any legacy unresolved fallback entries so they can re-resolve on new patches
    raw.into_iter()
        .filter(|(_, info)| !info.icon_url.is_empty() || info.rarity.is_some())
        .collect()
}

fn write_augment_resolution_cache(cache: &HashMap<String, MatchHistoryAugmentInfo>) {
    let path = augment_resolution_cache_path();
    if let Some(directory) = path.parent() {
        if let Err(error) = fs::create_dir_all(directory) {
            logger::error(
                "tauri.history",
                &format!("Failed creating augment cache directory: {error}"),
            );
            return;
        }
    }
    if let Err(error) = serde_json::to_vec(cache)
        .map_err(|error| error.to_string())
        .and_then(|bytes| fs::write(path, bytes).map_err(|error| error.to_string()))
    {
        logger::error(
            "tauri.history",
            &format!("Failed writing augment resolution cache: {error}"),
        );
    }
}

async fn fetch_game_detail(client: &LcuClient, game_id: u64) -> Option<Value> {
    let route = format!("/lol-match-history/v1/games/{game_id}");
    match client.get_json::<Value>(&route).await {
        Ok(detail) => {
            logger::debug(
                "tauri.history",
                &format!("Detail loaded for game {game_id}"),
            );
            log_history_response(&route, Some(game_id), &detail);
            write_detail_cache(game_id, &detail);
            Some(detail)
        }
        Err(error) => {
            logger::warn(
                "tauri.history",
                &format!("Detail unavailable for game {game_id}: {error}"),
            );
            None
        }
    }
}

fn normalize_augment_icon_url(icon_path: &str) -> String {
    let trimmed = icon_path.trim().replace(".tex", ".png");
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_lowercase();
    }

    let mut path = trimmed.to_lowercase();
    const PREFIX: &str = "/lol-game-data/assets/";
    if path.starts_with(PREFIX) {
        path = path[PREFIX.len()..].to_string();
    }

    loop {
        if let Some(stripped) = path.strip_prefix('/') {
            path = stripped.to_string();
        } else if let Some(stripped) = path.strip_prefix("assets/") {
            path = stripped.to_string();
        } else {
            break;
        }
    }

    format!("https://raw.communitydragon.org/latest/game/assets/{path}")
}
const BUNDLED_AUGMENT_TOOLTIPS_JSON: &str = include_str!("../data/augment_tooltips.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedItemInfo {
    pub name: String,
    pub icon_url: String,
    pub item_class: Option<String>,
    pub description: Option<String>,
    pub price_total: i64,
    pub categories: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AugmentCatalogEntry {
    name: String,
    icon_url: String,
    rarity: Option<String>,
    tooltip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSpellInfo {
    pub id: u64,
    pub name: String,
    pub icon_url: String,
    pub description: Option<String>,
}

lazy_static! {
    static ref ITEMS_CACHE: RwLock<Option<HashMap<i32, CachedItemInfo>>> = RwLock::new(None);
    static ref MAPS_CACHE: RwLock<Option<HashMap<u16, String>>> = RwLock::new(None);
    static ref AUGMENTS_CATALOG_CACHE: RwLock<Option<HashMap<String, AugmentCatalogEntry>>> =
        RwLock::new(None);
    static ref RESOLVED_AUGMENTS_CACHE: RwLock<Option<HashMap<String, MatchHistoryAugmentInfo>>> =
        RwLock::new(None);
    static ref SPELLS_CACHE: RwLock<Option<HashMap<u64, CachedSpellInfo>>> = RwLock::new(None);
    static ref MAYHEM_CHAMPION_BUILDS_CACHE: RwLock<HashMap<(i64, Vec<String>, String), (Instant, MayhemChampionBuildSummary)>> =
        RwLock::new(HashMap::new());
    static ref MAYHEM_CHAMPION_AUGMENTS_CACHE: RwLock<HashMap<(i64, Vec<String>), (Instant, f64, usize, HashMap<String, (usize, usize)>)>> =
        RwLock::new(HashMap::new());
    static ref MAYHEM_GLOBAL_AUGMENTS_CACHE: RwLock<Option<(Instant, usize, HashMap<String, (usize, f64, f64)>)>> =
        RwLock::new(None);
}

pub fn invalidate_mayhem_builds_cache() {
    if let Ok(mut cache) = MAYHEM_CHAMPION_BUILDS_CACHE.try_write() {
        cache.clear();
    }
    if let Ok(mut cache) = MAYHEM_CHAMPION_AUGMENTS_CACHE.try_write() {
        cache.clear();
    }
    if let Ok(mut cache) = MAYHEM_GLOBAL_AUGMENTS_CACHE.try_write() {
        *cache = None;
    }
}

fn default_map_name(map_id: u16) -> Option<String> {
    match map_id {
        11 => Some("Summoner's Rift".to_string()),
        12 => Some("Howling Abyss".to_string()),
        14 => Some("Butcher's Bridge".to_string()),
        21 => Some("Nexus Blitz".to_string()),
        30 => Some("Koeshin's Crossing".to_string()),
        33 => Some("Bridge of Progress".to_string()),
        35 => Some("Brawl".to_string()),
        _ => None,
    }
}

async fn get_maps_by_id() -> HashMap<u16, String> {
    let _ = crate::services::patch_manager::ensure_patch_freshness(None).await;
    {
        let cache = MAPS_CACHE.read().await;
        if let Some(maps) = cache.as_ref() {
            return maps.clone();
        }
    }

    let mut maps = HashMap::new();
    for id in [11u16, 12, 14, 21, 30, 33, 35] {
        if let Some(name) = default_map_name(id) {
            maps.insert(id, name);
        }
    }

    if let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
    {
        if let Ok(resp) = client.get(CDRAGON_MAPS_URL).send().await {
            if let Ok(raw_maps) = resp.json::<Vec<Value>>().await {
                for m in raw_maps {
                    if let Some(id) = m.get("id").and_then(Value::as_u64).map(|v| v as u16) {
                        if let Some(name) = m
                            .get("name")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                        {
                            maps.insert(id, name.to_string());
                        }
                    }
                }
            }
        }
    }

    let mut cache = MAPS_CACHE.write().await;
    *cache = Some(maps.clone());
    maps
}

pub async fn get_spells_by_id() -> Result<HashMap<u64, CachedSpellInfo>, String> {
    let _ = crate::services::patch_manager::ensure_patch_freshness(None).await;
    {
        let cache = SPELLS_CACHE.read().await;
        if let Some(spells) = cache.as_ref() {
            return Ok(spells.clone());
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .map_err(|e| format!("Failed building reqwest client for spells: {e}"))?;

    let raw_spells: Vec<Value> = client
        .get(CDRAGON_SUMMONER_SPELLS_URL)
        .send()
        .await
        .map_err(|e| format!("Failed fetching summoner spells: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed parsing summoner spells: {e}"))?;

    let mut spells = HashMap::new();
    for s in raw_spells {
        let Some(id) = s.get("id").and_then(Value::as_u64) else {
            continue;
        };
        let name = s
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if name.trim().is_empty() {
            continue;
        }
        let raw_icon = s.get("iconPath").and_then(Value::as_str).unwrap_or("");
        let icon_file = raw_icon.rsplit('/').next().unwrap_or("").to_lowercase();
        let icon_url = if !icon_file.is_empty() {
            format!(
                "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/data/spells/icons2d/{}",
                icon_file
            )
        } else {
            String::new()
        };
        let description = s
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string);
        spells.insert(
            id,
            CachedSpellInfo {
                id,
                name,
                icon_url,
                description,
            },
        );
    }

    let mut cache = SPELLS_CACHE.write().await;
    *cache = Some(spells.clone());
    Ok(spells)
}

fn classify_item(
    name: &str,
    price_total: i64,
    from_count: usize,
    categories: &[String],
    desc: &str,
) -> Option<String> {
    if categories.iter().any(|c| c.eq_ignore_ascii_case("boots")) {
        return Some("boot".to_string());
    }
    if desc.contains("<rarityLegendary>") || desc.contains("<rarityMythic>") {
        return Some("legendary".to_string());
    }
    if name.contains("Doran")
        || name.contains("Guardian's")
        || name.contains("Tear of the Goddess")
        || (price_total <= 500
            && categories
                .iter()
                .any(|c| c.eq_ignore_ascii_case("lane") || c.eq_ignore_ascii_case("jungle")))
    {
        return Some("starter".to_string());
    }
    if categories
        .iter()
        .any(|c| c.eq_ignore_ascii_case("consumable") || c.eq_ignore_ascii_case("trinket"))
    {
        return Some("other".to_string());
    }
    if from_count > 0 && price_total >= 2200 {
        return Some("legendary".to_string());
    }
    if price_total > 0 {
        return Some("component".to_string());
    }
    None
}

async fn get_items_by_id() -> Result<HashMap<i32, CachedItemInfo>, String> {
    let _ = crate::services::patch_manager::ensure_patch_freshness(None).await;
    {
        let cache = ITEMS_CACHE.read().await;
        if let Some(items) = cache.as_ref() {
            return Ok(items.clone());
        }
    }

    let raw_items: Vec<Value> = reqwest::Client::new()
        .get(CDRAGON_ITEMS_URL)
        .send()
        .await
        .map_err(|error| format!("Failed fetching item catalog: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Failed parsing item catalog: {error}"))?;

    let mut items = HashMap::new();
    for item in raw_items {
        let Some(id) = item.get("id").and_then(Value::as_i64).map(|id| id as i32) else {
            continue;
        };
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let Some(icon_path) = item.get("iconPath").and_then(Value::as_str) else {
            continue;
        };
        let Some(icon_name) = icon_path.rsplit('/').next() else {
            continue;
        };
        let normalized_icon_path = icon_path.to_lowercase();
        let icon_url = if normalized_icon_path.contains("/kiwi/") {
            format!(
                "{CDRAGON_GAME_ITEM_ICON_BASE}/kiwi/{}",
                icon_name.to_lowercase()
            )
        } else {
            format!("{CDRAGON_ITEM_ICON_BASE}/{}", icon_name.to_lowercase())
        };

        let price_total = item.get("priceTotal").and_then(Value::as_i64).unwrap_or(0);
        let from_count = item
            .get("from")
            .and_then(Value::as_array)
            .map(|a| a.len())
            .unwrap_or(0);
        let categories: Vec<String> = item
            .get("categories")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|c| c.as_str().map(str::to_string))
            .collect();
        let raw_desc = item
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let item_class = classify_item(&name, price_total, from_count, &categories, raw_desc);
        let description = if raw_desc.is_empty() {
            None
        } else {
            Some(raw_desc.to_string())
        };

        items.insert(
            id,
            CachedItemInfo {
                name,
                icon_url,
                item_class,
                description,
                price_total,
                categories,
            },
        );
    }

    let mut cache = ITEMS_CACHE.write().await;
    *cache = Some(items.clone());
    Ok(items)
}

fn clean_num_str(num: f64) -> String {
    if !num.is_finite() {
        return "0".to_string();
    }
    let rounded = (num * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 1e-6 {
        format!("{}", rounded.round() as i64)
    } else {
        format!("{rounded}")
    }
}

fn stat_name_from_id(stat: i64) -> &'static str {
    match stat {
        1 => "AP",
        2 => "AD",
        3 => "Armor",
        4 => "MR",
        5 => "Health",
        7 => "MS",
        8 => "AS",
        12 => "max Health",
        _ => "Adaptive Force",
    }
}

fn format_kiwi_calculation(
    calc: &Value,
    val_map: &HashMap<String, f64>,
    all_calcs: &serde_json::Map<String, Value>,
    depth: u8,
) -> String {
    if depth > 4 {
        return String::new();
    }
    if let Some(calc_key) = calc.as_str() {
        if let Some(target) = all_calcs.get(calc_key) {
            return format_kiwi_calculation(target, val_map, all_calcs, depth + 1);
        }
        return String::new();
    }
    let Some(obj) = calc.as_object() else {
        return String::new();
    };
    let is_pct = obj
        .get("mDisplayAsPercent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let calc_type = obj.get("__type").and_then(Value::as_str).unwrap_or("");

    if calc_type == "GameCalculationModified" {
        let mult = obj
            .get("mMultiplier")
            .and_then(|m| m.get("mNumber"))
            .and_then(Value::as_f64)
            .unwrap_or(1.0);
        if let Some(sub_calc) = obj.get("mModifiedGameCalculation") {
            let sub_str = format_kiwi_calculation(sub_calc, val_map, all_calcs, depth + 1);
            if let Ok(sub_num) = sub_str.trim_end_matches('%').parse::<f64>() {
                let pct_suffix = if sub_str.ends_with('%') || is_pct {
                    "%"
                } else {
                    ""
                };
                return format!("{}{pct_suffix}", clean_num_str(sub_num * mult));
            }
            if (mult - 1.0).abs() < 1e-6 {
                return sub_str;
            }
            return format!("{sub_str} * {}", clean_num_str(mult));
        }
    }

    if let Some(parts) = obj.get("mFormulaParts").and_then(Value::as_array) {
        let mut base_parts = Vec::new();
        let mut scaling_parts = Vec::new();

        for part in parts {
            let Some(p_obj) = part.as_object() else {
                continue;
            };
            let p_type = p_obj.get("__type").and_then(Value::as_str).unwrap_or("");
            match p_type {
                "ByCharLevelBreakpointsCalculationPart"
                | "ByCharLevelInterpolationCalculationPart" => {
                    let mut lvl1 = p_obj
                        .get("mLevel1Value")
                        .or_else(|| p_obj.get("mStartValue"))
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let per_lvl = p_obj
                        .get("mInitialBonusPerLevel")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let mut end_val = p_obj
                        .get("mEndValue")
                        .and_then(Value::as_f64)
                        .unwrap_or(lvl1 + 17.0 * per_lvl);
                    if is_pct {
                        lvl1 *= 100.0;
                        end_val *= 100.0;
                    }
                    let s1 = clean_num_str(lvl1);
                    let s2 = clean_num_str(end_val);
                    let suffix = if is_pct { "%" } else { "" };
                    if s1 == s2 {
                        base_parts.push(format!("{s1}{suffix}"));
                    } else {
                        base_parts.push(format!("{s1}{suffix} - {s2}{suffix}"));
                    }
                }
                "StatByCoefficientCalculationPart" => {
                    let stat = p_obj.get("mStat").and_then(Value::as_i64).unwrap_or(0);
                    let coef = p_obj
                        .get("mCoefficient")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0)
                        * 100.0;
                    scaling_parts.push(format!(
                        "+{}% {}",
                        clean_num_str(coef),
                        stat_name_from_id(stat)
                    ));
                }
                "NumberCalculationPart" => {
                    if let Some(mut val) = p_obj.get("mNumber").and_then(Value::as_f64) {
                        if is_pct {
                            val *= 100.0;
                        }
                        let suffix = if is_pct { "%" } else { "" };
                        base_parts.push(format!("{}{suffix}", clean_num_str(val)));
                    }
                }
                "NamedDataValueCalculationPart" => {
                    if let Some(dv_name) = p_obj.get("mDataValue").and_then(Value::as_str) {
                        if let Some(&raw_val) = val_map.get(&dv_name.to_lowercase()) {
                            let val = if is_pct { raw_val * 100.0 } else { raw_val };
                            let suffix = if is_pct { "%" } else { "" };
                            base_parts.push(format!("{}{suffix}", clean_num_str(val)));
                        }
                    }
                }
                "StatByNamedDataValueCalculationPart" => {
                    if let Some(dv_name) = p_obj.get("mDataValue").and_then(Value::as_str) {
                        let val = val_map.get(&dv_name.to_lowercase()).copied().unwrap_or(0.0);
                        let stat = p_obj.get("mStat").and_then(Value::as_i64).unwrap_or(0);
                        scaling_parts.push(format!(
                            "+{}% {}",
                            clean_num_str(val * 100.0),
                            stat_name_from_id(stat)
                        ));
                    }
                }
                "SumOfSubPartsCalculationPart" => {
                    if let Some(subs) = p_obj.get("mSubparts").and_then(Value::as_array) {
                        for sub in subs {
                            if let Some(mut val) = sub.get("mNumber").and_then(Value::as_f64) {
                                if is_pct {
                                    val *= 100.0;
                                }
                                let suffix = if is_pct { "%" } else { "" };
                                base_parts.push(format!("{}{suffix}", clean_num_str(val)));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let mut res = base_parts.join(" + ");
        if !scaling_parts.is_empty() {
            if res.is_empty() {
                res = format!("({})", scaling_parts.join(", "));
            } else {
                res = format!("{res} ({})", scaling_parts.join(", "));
            }
        }
        return res;
    }

    String::new()
}

fn replace_math_tokens_in_tooltip(
    template: &str,
    val_map: &HashMap<String, f64>,
    calcs: Option<&serde_json::Map<String, Value>>,
    stringtable: &HashMap<String, String>,
) -> String {
    // 1. Replace {{Macro}} tokens from stringtable
    let mut with_macros = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        with_macros.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        if let Some(end) = after_start.find("}}") {
            let key = after_start[..end].trim().to_lowercase();
            if key == "item_keyword_onhit" {
                with_macros.push_str("On-Hit");
            } else if key == "spellname" || key == "spellnameonly" {
                with_macros.push_str("Ability");
            } else if let Some(val) = stringtable.get(&key) {
                with_macros.push_str(val);
            } else {
                with_macros.push_str(&rest[start..start + 2 + end + 2]);
            }
            rest = &after_start[end + 2..];
        } else {
            with_macros.push_str(&rest[start..]);
            rest = "";
            break;
        }
    }
    with_macros.push_str(rest);

    // 2. Replace @Param*Multiplier@ / @Param@ tokens
    let mut result = String::with_capacity(with_macros.len());
    let chars: Vec<char> = with_macros.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '@' {
            if let Some(end_offset) = chars[i + 1..].iter().position(|&c| c == '@') {
                let end_idx = i + 1 + end_offset;
                let expr: String = chars[i + 1..end_idx].iter().collect();
                let trailing_pct = end_idx + 1 < chars.len() && chars[end_idx + 1] == '%';

                // Check for @f1@, @f2.0@ runtime trackers
                let expr_lower = expr.to_lowercase();
                if expr_lower.starts_with('f')
                    && expr_lower[1..]
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '.' || c == '*')
                {
                    i = if trailing_pct {
                        end_idx + 2
                    } else {
                        end_idx + 1
                    };
                    continue;
                }

                // Parse param and optional operator (*, /, +, -)
                let op_pos = expr
                    .char_indices()
                    .skip(1)
                    .find(|&(_, c)| c == '*' || c == '/' || c == '+' || c == '-');
                let (param, op_and_num) = if let Some((pos, op)) = op_pos {
                    (
                        &expr[..pos],
                        Some((op, expr[pos + 1..].parse::<f64>().ok())),
                    )
                } else {
                    (expr.as_str(), None)
                };

                let p_lower = param.to_lowercase();
                let var_name = if let Some((_, sub)) = p_lower.split_once(':') {
                    sub
                } else {
                    &p_lower
                };
                let mut replaced: Option<String> = None;

                if let Some(&base_val) = val_map.get(&p_lower).or_else(|| val_map.get(var_name)) {
                    let mut val = base_val;
                    if let Some((op, Some(operand))) = op_and_num {
                        match op {
                            '*' => val *= operand,
                            '/' if operand.abs() > f64::EPSILON => val /= operand,
                            '+' => val += operand,
                            '-' => val -= operand,
                            _ => {}
                        }
                    }
                    let pct_str = if trailing_pct { "%" } else { "" };
                    replaced = Some(format!("{}{pct_str}", clean_num_str(val)));
                } else if let Some(calcs_map) = calcs {
                    if let Some((_, calc_obj)) = calcs_map.iter().find(|(k, _)| {
                        let kl = k.to_lowercase();
                        kl == p_lower || kl == var_name
                    }) {
                        let formatted = format_kiwi_calculation(calc_obj, val_map, calcs_map, 0);
                        if !formatted.is_empty() {
                            if formatted.ends_with('%') && trailing_pct {
                                replaced = Some(formatted);
                            } else {
                                let pct_str = if trailing_pct { "%" } else { "" };
                                replaced = Some(format!("{formatted}{pct_str}"));
                            }
                        }
                    }
                }

                if let Some(rep) = replaced {
                    result.push_str(&rep);
                    i = if trailing_pct {
                        end_idx + 2
                    } else {
                        end_idx + 1
                    };
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

fn resolve_live_kiwi_tooltip(
    name_id: &str,
    cherry_desc: Option<&str>,
    kiwi_root: Option<&Value>,
    stringtable: &HashMap<String, String>,
) -> Option<String> {
    let name_lower = name_id
        .trim_start_matches("ARAM_")
        .trim_start_matches("aram_")
        .to_lowercase();

    let kiwi_obj = kiwi_root.and_then(Value::as_object);
    let kiwi_entry = kiwi_obj.and_then(|root| {
        root.get(&format!("Maps/ModeSpecificData/Augments/{name_id}"))
            .or_else(|| root.get(&format!("Maps/ModeSpecificData/Augments/ARAM_{name_id}")))
            .or_else(|| {
                root.values().find(|v| {
                    v.get("AugmentNameId")
                        .and_then(Value::as_str)
                        .map(|s| s.eq_ignore_ascii_case(name_id))
                        .unwrap_or(false)
                })
            })
    });

    let mut raw_tooltip = String::new();
    if let Some(entry) = kiwi_entry {
        let tooltip_key = entry
            .get("AugmentTooltipTra")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let desc_key = entry
            .get("DescriptionTra")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        if let Some(t) = stringtable.get(&tooltip_key) {
            raw_tooltip = t.clone();
        }
        if raw_tooltip.is_empty() || raw_tooltip.starts_with("{{") {
            if let Some(d) = stringtable
                .get(&desc_key)
                .or_else(|| stringtable.get(&format!("cherry_{name_lower}_summary")))
                .or_else(|| stringtable.get(&format!("kiwi_{name_lower}_summary")))
            {
                raw_tooltip = d.clone();
            }
        }
    }

    if raw_tooltip.is_empty() && !name_lower.is_empty() {
        if let Some(t) = stringtable
            .get(&format!("cherry_{name_lower}_tooltip"))
            .or_else(|| stringtable.get(&format!("kiwi_{name_lower}_tooltip")))
            .or_else(|| stringtable.get(&format!("cherry_{name_lower}_summary")))
            .or_else(|| stringtable.get(&format!("kiwi_{name_lower}_summary")))
        {
            raw_tooltip = t.clone();
        }
    }

    if raw_tooltip.is_empty() {
        if let Some(desc) = cherry_desc.filter(|s| !s.trim().is_empty()) {
            raw_tooltip = desc.to_string();
        }
    }

    if raw_tooltip.contains("<mainText>") {
        if let Some(start) = raw_tooltip.find("<mainText>") {
            let rest = &raw_tooltip[start + 10..];
            if let Some(end) = rest.find("</mainText>") {
                raw_tooltip = rest[..end].to_string();
            }
        }
    }

    if raw_tooltip.is_empty() {
        return None;
    }

    let mut val_map = HashMap::new();
    let mut calcs_map = None;

    if let (Some(root), Some(entry)) = (kiwi_obj, kiwi_entry) {
        if let Some(root_spell_key) = entry.get("RootSpell").and_then(Value::as_str) {
            if let Some(m_spell) = root.get(root_spell_key).and_then(|s| s.get("mSpell")) {
                if let Some(dvs) = m_spell.get("DataValues").and_then(Value::as_array) {
                    for dv in dvs {
                        if let (Some(n), Some(vals)) = (
                            dv.get("name").and_then(Value::as_str),
                            dv.get("values").and_then(Value::as_array),
                        ) {
                            let val = vals
                                .first()
                                .or_else(|| vals.get(1))
                                .and_then(Value::as_f64)
                                .unwrap_or(0.0);
                            val_map.insert(n.to_lowercase(), val);
                        }
                    }
                }
                calcs_map = m_spell.get("mSpellCalculations").and_then(Value::as_object);
            }
        }
    }

    let resolved = replace_math_tokens_in_tooltip(&raw_tooltip, &val_map, calcs_map, stringtable);
    let trimmed = resolved.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

async fn get_augment_catalog_map() -> Result<HashMap<String, AugmentCatalogEntry>, String> {
    let _ = crate::services::patch_manager::ensure_patch_freshness(None).await;
    {
        let cache = AUGMENTS_CATALOG_CACHE.read().await;
        if let Some(catalog) = cache.as_ref() {
            return Ok(catalog.clone());
        }
    }

    // Check per-patch disk cache first
    let catalog_disk_path = augment_catalog_cache_path();
    if let Ok(bytes) = fs::read(&catalog_disk_path) {
        if let Ok(disk_catalog) =
            serde_json::from_slice::<HashMap<String, AugmentCatalogEntry>>(&bytes)
        {
            if !disk_catalog.is_empty() {
                let mut cache = AUGMENTS_CATALOG_CACHE.write().await;
                *cache = Some(disk_catalog.clone());
                return Ok(disk_catalog);
            }
        }
    }

    let tooltips_map: HashMap<String, Value> =
        serde_json::from_str(BUNDLED_AUGMENT_TOOLTIPS_JSON).unwrap_or_default();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;

    let mut catalog_map: HashMap<String, AugmentCatalogEntry> = HashMap::new();

    let get_bundled_tooltip = |id: Option<&str>, name_id: Option<&str>| -> Option<String> {
        id.and_then(|i| tooltips_map.get(i))
            .or_else(|| name_id.and_then(|n| tooltips_map.get(n)))
            .or_else(|| name_id.and_then(|n| tooltips_map.get(&n.to_lowercase())))
            .or_else(|| {
                name_id.and_then(|n| n.strip_prefix("ARAM_")).and_then(|s| {
                    tooltips_map
                        .get(s)
                        .or_else(|| tooltips_map.get(&s.to_lowercase()))
                })
            })
            .and_then(|v| v.get("tooltip"))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };

    // Fetch cherry-augments.json, kiwi.bin.json, and lol.stringtable.json concurrently
    let (cherry_res, kiwi_res, stringtable_res) = tokio::join!(
        async {
            client
                .get(CDRAGON_CHERRY_AUGMENTS_URL)
                .send()
                .await
                .ok()?
                .json::<Vec<Value>>()
                .await
                .ok()
        },
        async {
            client
                .get(CDRAGON_KIWI_AUGMENTS_URL)
                .send()
                .await
                .ok()?
                .json::<Value>()
                .await
                .ok()
        },
        async {
            client
                .get(CDRAGON_STRINGTABLE_URL)
                .send()
                .await
                .ok()?
                .json::<Value>()
                .await
                .ok()
        }
    );

    let mut stringtable_entries: HashMap<String, String> = HashMap::new();
    let mut stringtable_titles: HashMap<String, String> = HashMap::new();
    if let Some(st_val) = stringtable_res {
        if let Some(entries_obj) = st_val.get("entries").and_then(Value::as_object) {
            for (k, v) in entries_obj {
                if let Some(s) = v.as_str() {
                    stringtable_entries.insert(k.to_lowercase(), s.to_string());
                    if s.contains("<titleLeft>") && s.contains("<mainText>") {
                        if let Some(start_title) = s.find("<titleLeft>") {
                            let title_slice = &s[start_title + 11..];
                            if let Some(end_title) = title_slice.find("</titleLeft>") {
                                let clean_title = title_slice[..end_title]
                                    .replace("&nbsp;", " ")
                                    .trim()
                                    .to_string();
                                if let Some(start_main) = s.find("<mainText>") {
                                    let main_slice = &s[start_main + 10..];
                                    if let Some(end_main) = main_slice.find("</mainText>") {
                                        let desc = main_slice[..end_main].trim().to_string();
                                        if !desc.is_empty() {
                                            stringtable_titles
                                                .insert(clean_title.to_lowercase(), desc);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 1. Primary: cherry-augments.json (Contains official localized names and paths for Arena and ARAM Mayhem)
    if let Some(entries) = cherry_res {
        for entry in entries {
            let mut name = entry
                .get("nameTRA")
                .or_else(|| entry.get("name"))
                .or_else(|| entry.get("simpleNameTRA"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();

            let id_str = entry
                .get("id")
                .and_then(Value::as_i64)
                .map(|id| id.to_string());
            let name_id_str = entry.get("augmentNameId").and_then(Value::as_str);

            if name.is_empty() {
                if let Some(nid) = name_id_str {
                    if nid == "GoH_PentakillAug_Generic" {
                        name = "Gifts of Heavy Metal".to_string();
                    } else if let Some(stripped) = nid
                        .strip_prefix("ARAM_")
                        .or_else(|| nid.strip_prefix("GoH_"))
                    {
                        name = stripped.replace('_', " ");
                    } else {
                        name = nid.replace('_', " ");
                    }
                } else if let Some(ref id) = id_str {
                    name = format!("Augment #{id}");
                } else {
                    continue;
                }
            }

            let icon_path = entry
                .get("augmentSmallIconPath")
                .or_else(|| entry.get("augmentLargeIconPath"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let icon_url = normalize_augment_icon_url(icon_path);
            let raw_rarity = entry.get("rarity").and_then(Value::as_str).unwrap_or("");
            let rarity = match raw_rarity.to_lowercase().as_str() {
                "ksilver" | "silver" => Some("silver".to_string()),
                "kgold" | "gold" => Some("gold".to_string()),
                "kprismatic" | "prismatic" => Some("prismatic".to_string()),
                _ => None,
            };

            let cherry_desc = entry
                .get("desc")
                .or_else(|| entry.get("tooltip"))
                .and_then(Value::as_str);

            let live_tooltip = name_id_str.and_then(|nid| {
                resolve_live_kiwi_tooltip(nid, cherry_desc, kiwi_res.as_ref(), &stringtable_entries)
            });
            let bundled_tooltip = get_bundled_tooltip(id_str.as_deref(), name_id_str);
            let title_tooltip = stringtable_titles
                .get(&name.to_lowercase())
                .or_else(|| name_id_str.and_then(|nid| stringtable_titles.get(&nid.to_lowercase())))
                .cloned();

            // Prefer live CDragon tooltip if it resolved cleanly without unresolved @ tokens; otherwise fallback to bundled, title map, or cherry_desc
            let tooltip = match (&live_tooltip, &bundled_tooltip, &title_tooltip) {
                (Some(live), _, _) if !live.contains('@') && !live.contains("{{") => {
                    Some(live.clone())
                }
                (_, Some(bundled), _) => Some(bundled.clone()),
                (Some(live), None, _) if !live.trim().is_empty() => Some(live.clone()),
                (_, _, Some(t_desc)) => Some(t_desc.clone()),
                _ => cherry_desc.map(str::to_string),
            };

            let parsed_entry = AugmentCatalogEntry {
                name: name.clone(),
                icon_url,
                rarity,
                tooltip,
            };

            // Index by integer ID as string
            if let Some(id) = entry.get("id").and_then(Value::as_i64) {
                catalog_map.insert(id.to_string(), parsed_entry.clone());
            }

            // Index by augmentNameId (e.g. "ARAM_Zealot", "Zealot")
            if let Some(name_id) = name_id_str {
                catalog_map.insert(name_id.to_string(), parsed_entry.clone());
                catalog_map.insert(name_id.to_lowercase(), parsed_entry.clone());
                if let Some(stripped) = name_id.strip_prefix("ARAM_") {
                    catalog_map.insert(stripped.to_string(), parsed_entry.clone());
                    catalog_map.insert(stripped.to_lowercase(), parsed_entry.clone());
                }
            }
        }
    }

    // 2. Secondary fallback: kiwi.bin.json for any mode-specific augments not in cherry-augments.json
    if let Some(ref kiwi_val) = kiwi_res {
        populate_kiwi_catalog(
            kiwi_val,
            kiwi_val,
            &stringtable_entries,
            &stringtable_titles,
            &mut catalog_map,
            &tooltips_map,
        );
    }

    if !catalog_map.is_empty() {
        if let Some(parent) = catalog_disk_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec(&catalog_map) {
            let _ = fs::write(&catalog_disk_path, bytes);
        }
    }

    let mut cache = AUGMENTS_CATALOG_CACHE.write().await;
    *cache = Some(catalog_map.clone());
    Ok(catalog_map)
}

fn populate_kiwi_catalog(
    val: &Value,
    kiwi_root: &Value,
    stringtable: &HashMap<String, String>,
    stringtable_titles: &HashMap<String, String>,
    map: &mut HashMap<String, AugmentCatalogEntry>,
    tooltips_map: &HashMap<String, Value>,
) {
    match val {
        Value::Object(obj) => {
            let has_id = obj
                .get("id")
                .or_else(|| obj.get("AugmentPlatformId"))
                .or_else(|| obj.get("mName"))
                .or_else(|| obj.get("AugmentNameId"));
            if let Some(id_val) = has_id {
                let id_str = value_to_id(id_val).unwrap_or_default();
                if !id_str.is_empty() && !map.contains_key(&id_str) {
                    let display_name_key = obj
                        .get("DisplayNameTra")
                        .or_else(|| obj.get("NameTra"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_lowercase();
                    let name = stringtable
                        .get(&display_name_key)
                        .cloned()
                        .or_else(|| {
                            obj.get("name")
                                .or_else(|| obj.get("mName"))
                                .or_else(|| obj.get("AugmentNameId"))
                                .and_then(Value::as_str)
                                .map(|s| {
                                    if s == "GoH_PentakillAug_Generic" {
                                        "Gifts of Heavy Metal".to_string()
                                    } else {
                                        s.to_string()
                                    }
                                })
                        })
                        .unwrap_or_else(|| id_str.clone());
                    let icon_path = obj
                        .get("AugmentSmallIconPath")
                        .or_else(|| obj.get("AugmentLargeIconPath"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let icon_url = normalize_augment_icon_url(icon_path);
                    let live_tooltip =
                        resolve_live_kiwi_tooltip(&id_str, None, Some(kiwi_root), stringtable);
                    let bundled_tooltip = tooltips_map
                        .get(&id_str)
                        .or_else(|| tooltips_map.get(&id_str.to_lowercase()))
                        .and_then(|v| v.get("tooltip"))
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                    let title_tooltip = stringtable_titles.get(&name.to_lowercase()).cloned();
                    let tooltip = live_tooltip.or(bundled_tooltip).or(title_tooltip);
                    let rarity = obj.get("rarity").and_then(|r| match r {
                        Value::Number(n) => match n.as_i64() {
                            Some(0) => Some("silver".to_string()),
                            Some(1) => Some("gold".to_string()),
                            Some(2) => Some("prismatic".to_string()),
                            _ => None,
                        },
                        Value::String(s) => match s.to_lowercase().as_str() {
                            "ksilver" | "silver" => Some("silver".to_string()),
                            "kgold" | "gold" => Some("gold".to_string()),
                            "kprismatic" | "prismatic" => Some("prismatic".to_string()),
                            _ => None,
                        },
                        _ => None,
                    });
                    let entry = AugmentCatalogEntry {
                        name,
                        icon_url,
                        rarity,
                        tooltip,
                    };
                    map.insert(id_str.clone(), entry.clone());
                    map.insert(id_str.to_lowercase(), entry.clone());
                    if let Some(pid) = obj.get("AugmentPlatformId").and_then(value_to_id) {
                        map.insert(pid.clone(), entry.clone());
                    }
                    if let Some(nid) = obj.get("AugmentNameId").and_then(Value::as_str) {
                        map.insert(nid.to_string(), entry.clone());
                        map.insert(nid.to_lowercase(), entry);
                    }
                }
            }
            for child in obj.values() {
                populate_kiwi_catalog(
                    child,
                    kiwi_root,
                    stringtable,
                    stringtable_titles,
                    map,
                    tooltips_map,
                );
            }
        }
        Value::Array(arr) => {
            for child in arr {
                populate_kiwi_catalog(
                    child,
                    kiwi_root,
                    stringtable,
                    stringtable_titles,
                    map,
                    tooltips_map,
                );
            }
        }
        _ => {}
    }
}

fn value_to_id(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_i64().map(|number| number.to_string()))
}

async fn resolve_augments(raw: Option<&Value>) -> Vec<MatchHistoryAugmentInfo> {
    let Some(Value::Array(raw_augments)) = raw else {
        return Vec::new();
    };
    let ids: Vec<String> = raw_augments
        .iter()
        .filter_map(|augment| match augment {
            Value::String(id) => Some(id.clone()),
            Value::Object(object) => ["id", "mName", "AugmentNameId", "AugmentPlatformId"]
                .iter()
                .filter_map(|key| object.get(*key))
                .filter_map(value_to_id)
                .next(),
            _ => None,
        })
        .collect();
    if ids.is_empty() {
        return Vec::new();
    }
    let cached_augments = {
        let mut cache = RESOLVED_AUGMENTS_CACHE.write().await;
        if cache.is_none() {
            *cache = Some(read_augment_resolution_cache());
        }
        cache.clone().unwrap_or_default()
    };
    let mut resolved = Vec::new();
    let mut unresolved_ids = Vec::new();
    for id in ids {
        if let Some(cached) = cached_augments.get(&id) {
            resolved.push(cached.clone());
        } else if !unresolved_ids.contains(&id) {
            unresolved_ids.push(id);
        }
    }
    if unresolved_ids.is_empty() {
        logger::debug(
            "tauri.history",
            &format!(
                "Augment resolution cache hit for {} augments",
                resolved.len()
            ),
        );
        return resolved;
    }

    if !unresolved_ids.is_empty() {
        let Ok(catalog) = get_augment_catalog_map().await else {
            logger::warn("tauri.history", "Augment catalog unavailable");
            return resolved;
        };

        let mut newly_resolved = Vec::new();
        let mut cache_updates = Vec::new();
        for id in unresolved_ids {
            let matched = catalog
                .get(&id)
                .or_else(|| catalog.get(&id.to_lowercase()))
                .or_else(|| {
                    if let Some(stripped) = id.strip_prefix("ARAM_") {
                        catalog
                            .get(stripped)
                            .or_else(|| catalog.get(&stripped.to_lowercase()))
                    } else {
                        None
                    }
                });

            if let Some(entry) = matched {
                let info = MatchHistoryAugmentInfo {
                    id: id.clone(),
                    name: entry.name.clone(),
                    icon_url: entry.icon_url.clone(),
                    rarity: entry.rarity.clone(),
                    tooltip: entry.tooltip.clone(),
                };
                cache_updates.push(info.clone());
                newly_resolved.push(info);
            } else {
                // Fallback: humanize name if possible rather than "ARAM_..."
                // Do not persist unresolved fallbacks to disk so new patches/catalog refreshes can resolve them.
                let fallback_name = if let Some(stripped) = id.strip_prefix("ARAM_") {
                    stripped.to_string()
                } else if id.chars().all(|c| c.is_ascii_digit()) {
                    format!("Augment #{id}")
                } else {
                    id.clone()
                };
                newly_resolved.push(MatchHistoryAugmentInfo {
                    id: id.clone(),
                    name: fallback_name,
                    icon_url: String::new(),
                    rarity: None,
                    tooltip: None,
                });
            }
        }

        if !cache_updates.is_empty() {
            let mut cache = RESOLVED_AUGMENTS_CACHE.write().await;
            let cache = cache.get_or_insert_with(HashMap::new);
            for augment in cache_updates {
                cache.insert(augment.id.clone(), augment);
            }
            write_augment_resolution_cache(cache);
        }
        resolved.extend(newly_resolved);
    }
    logger::debug(
        "tauri.history",
        &format!("Resolved {} augments", resolved.len()),
    );
    resolved
}

fn match_history_list_cache_path() -> PathBuf {
    get_cache_dir().join("last_match_history_list.json")
}

fn read_cached_match_history_list() -> Option<Vec<MatchHistoryItem>> {
    let path = match_history_list_cache_path();
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_cached_match_history_list(new_matches: &[MatchHistoryItem], is_first_page: bool) {
    let path = match_history_list_cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let to_write: Vec<MatchHistoryItem> = if is_first_page {
        let existing = read_cached_match_history_list().unwrap_or_default();
        let mut seen: HashSet<u64> = new_matches.iter().map(|m| m.game_id).collect();
        let mut combined = new_matches.to_vec();
        for item in existing {
            if seen.insert(item.game_id) {
                combined.push(item);
            }
        }
        combined
    } else {
        let mut existing = read_cached_match_history_list().unwrap_or_default();
        let mut seen: HashSet<u64> = existing.iter().map(|m| m.game_id).collect();
        for item in new_matches {
            if seen.insert(item.game_id) {
                existing.push(item.clone());
            }
        }
        existing
    };
    if let Ok(bytes) = serde_json::to_vec_pretty(&to_write) {
        let _ = fs::write(path, bytes);
    }
}

pub async fn get_current_summoner_match_history(
    beg_index: u32,
    end_index: u32,
) -> Result<Vec<MatchHistoryItem>, String> {
    if LcuClient::find_credentials().is_none() {
        if let Some(cached) = read_cached_match_history_list() {
            if beg_index == 0 {
                return Ok(cached);
            } else {
                return Ok(Vec::new());
            }
        }
        return Err("League Client is not running. Please launch League of Legends.".to_string());
    }

    match fetch_live_summoner_match_history(beg_index, end_index).await {
        Ok(history_items) => {
            write_cached_match_history_list(&history_items, beg_index == 0);
            Ok(history_items)
        }
        Err(err) => {
            if let Some(cached) = read_cached_match_history_list() {
                if beg_index == 0 {
                    return Ok(cached);
                } else {
                    return Ok(Vec::new());
                }
            }
            Err(err)
        }
    }
}

async fn fetch_live_summoner_match_history(
    beg_index: u32,
    end_index: u32,
) -> Result<Vec<MatchHistoryItem>, String> {
    logger::info(
        "tauri.history",
        &format!("Starting fetch ({beg_index}..{end_index})"),
    );
    let client = LcuClient::connect()?;
    let _ = crate::services::patch_manager::ensure_patch_freshness(Some(&client)).await;
    logger::debug("tauri.history", "Loading champion metadata");
    let champions_by_id: HashMap<i64, ChampionSummary> = get_all_champions()
        .await?
        .into_iter()
        .map(|champion| (champion.id, champion))
        .collect();
    logger::debug("tauri.history", "Champion metadata ready");
    logger::debug("tauri.history", "Loading item and map metadata");
    let items_by_id = get_items_by_id().await?;
    let maps_by_id = get_maps_by_id().await;
    logger::debug(
        "tauri.history",
        &format!("Item metadata ready ({} entries)", items_by_id.len()),
    );

    let summoner: Value = client.get_json("/lol-summoner/v1/current-summoner").await?;
    let my_puuid = summoner
        .get("puuid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let my_account_id = summoner.get("accountId").and_then(|v| v.as_u64());

    let path = if my_puuid.is_empty() {
        format!(
            "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex={}&endIndex={}",
            beg_index, end_index
        )
    } else {
        format!(
            "/lol-match-history/v1/products/lol/{}/matches?begIndex={}&endIndex={}",
            my_puuid, beg_index, end_index
        )
    };
    logger::info(
        "tauri.history",
        &format!("Requesting match history from {path}"),
    );

    let raw_data: Value = client.get_json(&path).await?;
    log_history_response(&path, None, &raw_data);
    logger::debug("tauri.history", "LCU response received");
    let mut history_items = Vec::new();

    let games = raw_data
        .get("games")
        .and_then(|g| g.get("games"))
        .and_then(|g| g.as_array());

    let games = match games {
        Some(list) => list,
        None => return Ok(history_items),
    };

    let game_ids: Vec<u64> = games
        .iter()
        .filter_map(|game| game.get("gameId").and_then(Value::as_u64))
        .collect();
    let mut details_by_game_id = HashMap::new();
    let mut uncached_game_ids = Vec::new();
    for game_id in game_ids {
        let cached = tokio::task::spawn_blocking(move || read_detail_cache(game_id))
            .await
            .ok()
            .flatten();
        if let Some(detail) = cached {
            logger::debug(
                "tauri.history",
                &format!("Detail cache hit for game {game_id}"),
            );
            details_by_game_id.insert(game_id, detail);
        } else {
            uncached_game_ids.push(game_id);
        }
    }
    logger::debug(
        "tauri.history",
        &format!(
            "{} cached details, {} details to fetch in batches",
            details_by_game_id.len(),
            uncached_game_ids.len()
        ),
    );
    for (i, game_id) in uncached_game_ids.into_iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(75)).await;
        }
        if let Some(detail) = fetch_game_detail(&client, game_id).await {
            details_by_game_id.insert(game_id, detail);
        }
    }

    for game in games {
        let game_id = game.get("gameId").and_then(|v| v.as_u64()).unwrap_or(0);
        let game_mode = game
            .get("gameMode")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let game_duration_secs = game
            .get("gameDuration")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let game_creation_date = game
            .get("gameCreationDate")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let map_id = game.get("mapId").and_then(|v| v.as_u64()).unwrap_or(0) as u16;

        let participant_identities = game.get("participantIdentities").and_then(|p| p.as_array());
        let participants = game.get("participants").and_then(|p| p.as_array());

        let mut target_participant_id: Option<u64> = None;

        if let Some(identities) = participant_identities {
            for id_entry in identities {
                let p_id = id_entry
                    .get("participantId")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if let Some(player) = id_entry.get("player") {
                    let puuid = player.get("puuid").and_then(|v| v.as_str()).unwrap_or("");
                    let acc_id = player.get("accountId").and_then(|v| v.as_u64());

                    if (!my_puuid.is_empty() && puuid == my_puuid)
                        || (my_account_id.is_some() && acc_id == my_account_id)
                    {
                        target_participant_id = Some(p_id);
                        break;
                    }
                }
            }
        }

        if target_participant_id.is_none() {
            target_participant_id = Some(1);
        }

        let my_p_id = target_participant_id.unwrap_or(1);

        if let Some(parts) = participants {
            let matched_p = parts
                .iter()
                .find(|p| p.get("participantId").and_then(|v| v.as_u64()).unwrap_or(0) == my_p_id);

            if let Some(p) = matched_p {
                let champion_id = p.get("championId").and_then(|v| v.as_i64()).unwrap_or(0);
                let spell1_id = p.get("spell1Id").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
                let spell2_id = p.get("spell2Id").and_then(|v| v.as_u64()).unwrap_or(0) as u16;

                let stats = p.get("stats");
                let win = stats
                    .and_then(|s| s.get("win"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let kills = stats
                    .and_then(|s| s.get("kills"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let deaths = stats
                    .and_then(|s| s.get("deaths"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let assists = stats
                    .and_then(|s| s.get("assists"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let champ_level = stats
                    .and_then(|s| s.get("champLevel"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1);
                let minions_killed = stats
                    .and_then(|s| s.get("totalMinionsKilled"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
                    + stats
                        .and_then(|s| s.get("neutralMinionsKilled"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                let gold_earned = stats
                    .and_then(|s| s.get("goldEarned"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let mut damage_dealt_to_champions = stats
                    .and_then(|s| {
                        s.get("totalDamageDealtToChampions")
                            .or_else(|| s.get("damageDealtToChampions"))
                    })
                    .and_then(Value::as_i64);
                let mut augment_ids = Vec::new();

                if let Some(detail) = details_by_game_id.get(&game_id) {
                    if let Some(detail_participant) = detail
                        .get("participants")
                        .and_then(Value::as_array)
                        .and_then(|participants| {
                            participants
                                .iter()
                                .find(|participant| {
                                    participant.get("participantId").and_then(Value::as_u64)
                                        == Some(my_p_id)
                                })
                                .or_else(|| {
                                    participants.iter().find(|participant| {
                                        participant.get("championId").and_then(Value::as_i64)
                                            == Some(champion_id)
                                    })
                                })
                        })
                    {
                        let detail_stats = detail_participant.get("stats");
                        if let Some(detail_damage) = detail_stats
                            .and_then(|stats| stats.get("totalDamageDealtToChampions"))
                            .and_then(Value::as_i64)
                        {
                            damage_dealt_to_champions = Some(detail_damage);
                        }
                        if let Some(detail_stats) = detail_stats {
                            for index in 1..=6 {
                                let field = format!("playerAugment{index}");
                                if let Some(augment_id) = detail_stats.get(&field) {
                                    if let Some(augment_id) = value_to_id(augment_id) {
                                        if augment_id != "0" {
                                            augment_ids.push(Value::String(augment_id));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if augment_ids.is_empty() {
                    if let Some(raw_augments) = p.get("augments") {
                        if let Value::Array(raw_augments) = raw_augments {
                            augment_ids.extend(raw_augments.iter().cloned());
                        }
                    }
                }
                logger::debug(
                    "tauri.history",
                    &format!("Game {game_id} augment ids: {:?}", augment_ids),
                );
                let augments = resolve_augments(Some(&Value::Array(augment_ids))).await;

                let item_ids: Vec<i32> = (0..=6)
                    .map(|i| {
                        let item_key = format!("item{i}");
                        stats
                            .and_then(|s| s.get(&item_key))
                            .and_then(Value::as_i64)
                            .unwrap_or(0) as i32
                    })
                    .collect();
                let mut items = Vec::with_capacity(item_ids.len());
                for item_id in &item_ids {
                    if *item_id == 0 {
                        items.push(None);
                    } else if let Some(item_info) = items_by_id.get(item_id) {
                        items.push(Some(MatchHistoryItemInfo {
                            id: *item_id,
                            name: item_info.name.clone(),
                            icon_url: item_info.icon_url.clone(),
                            description: item_info.description.clone(),
                            price_total: Some(item_info.price_total),
                        }));
                    } else {
                        items.push(None);
                    }
                }

                let raw_version = game.get("gameVersion").and_then(Value::as_str).or_else(|| {
                    details_by_game_id
                        .get(&game_id)
                        .and_then(|d| d.get("gameVersion"))
                        .and_then(Value::as_str)
                });
                let patch = raw_version.and_then(crate::services::patch_manager::normalize_patch);
                let map_name = maps_by_id.get(&map_id).cloned();

                history_items.push(MatchHistoryItem {
                    game_id,
                    champion: champions_by_id.get(&champion_id).cloned(),
                    win,
                    game_mode,
                    game_duration_secs,
                    game_creation_date,
                    kills,
                    deaths,
                    assists,
                    champ_level,
                    minions_killed,
                    gold_earned,
                    spell1_id,
                    spell2_id,
                    items,
                    damage_dealt_to_champions,
                    augments,
                    map_id,
                    map_name,
                    patch,
                });
            }
        }
    }

    Ok(history_items)
}

pub async fn enrich_global_statistics(mut response: Value) -> Result<Value, String> {
    let facts = response
        .get_mut("facts")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "global statistics response did not contain facts".to_string())?;
    let items = get_items_by_id().await.unwrap_or_default();
    let champions = get_all_champions().await.unwrap_or_default();
    let champions_by_id: HashMap<i64, ChampionSummary> = champions
        .into_iter()
        .map(|champion| (champion.id, champion))
        .collect();
    let mut item_ids = std::collections::HashSet::new();
    let mut augment_ids = std::collections::HashSet::new();
    for fact in facts.iter() {
        if let Some(ids) = fact.get("item_ids").and_then(Value::as_array) {
            item_ids.extend(ids.iter().filter_map(Value::as_i64));
        }
        if let Some(ids) = fact.get("augment_ids").and_then(Value::as_array) {
            augment_ids.extend(ids.iter().filter_map(value_to_id));
        }
    }
    let item_details = join_all(item_ids.into_iter().filter_map(|id| {
        let item_info = items.get(&(id as i32))?.clone();
        Some(async move {
            (
                id,
                json!({
                    "id": id,
                    "name": item_info.name,
                    "icon_url": get_cached_asset_data_uri(&item_info.icon_url).await,
                    "description": item_info.description,
                    "price_total": item_info.price_total,
                    "item_class": item_info.item_class,
                }),
            )
        })
    }))
    .await
    .into_iter()
    .collect::<HashMap<_, _>>();
    let resolved_augments = resolve_augments(Some(&Value::Array(
        augment_ids.into_iter().map(Value::String).collect(),
    )))
    .await
    .into_iter()
    .map(|augment| (augment.id.clone(), augment))
    .collect::<HashMap<_, _>>();

    for fact in facts {
        let Some(object) = fact.as_object_mut() else {
            continue;
        };
        if let Some(champion_id) = object.get("champion_id").and_then(Value::as_i64) {
            if let Some(champion) = champions_by_id.get(&champion_id) {
                object.insert(
                    "champion".to_string(),
                    serde_json::to_value(champion).unwrap_or(Value::Null),
                );
            }
        }
        let item_ids = object
            .get("item_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let item_info = item_ids
            .into_iter()
            .filter_map(|id| id.as_i64().and_then(|id| item_details.get(&id).cloned()))
            .collect::<Vec<_>>();
        object.insert("items".to_string(), Value::Array(item_info));

        let augment_ids = object
            .get("augment_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let resolved = augment_ids
            .into_iter()
            .filter_map(|id| value_to_id(&id).and_then(|id| resolved_augments.get(&id).cloned()))
            .collect::<Vec<_>>();
        object.insert(
            "augments".to_string(),
            serde_json::to_value(resolved).unwrap_or_else(|_| json!([])),
        );
    }
    Ok(response)
}

pub async fn enrich_statistics_response(mut response: Value) -> Result<Value, String> {
    let items = get_items_by_id().await.unwrap_or_default();
    let champions = get_all_champions().await.unwrap_or_default();
    let champions_by_id: HashMap<i64, ChampionSummary> = champions
        .into_iter()
        .map(|champion| (champion.id as i64, champion))
        .collect();
    let rows = response
        .get_mut("rows")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "statistics response did not contain rows".to_string())?;
    let augment_ids = rows
        .iter()
        .flat_map(|row| {
            let mut ids = Vec::new();
            if let Some(id) = row.get("id").and_then(Value::as_str) {
                if id.starts_with("augment:") {
                    ids.push(id.trim_start_matches("augment:").to_string());
                } else if id.starts_with("combo:") {
                    for part in id.trim_start_matches("combo:").split('+') {
                        if let Some(aug_id) = part.strip_prefix("augment:") {
                            ids.push(aug_id.to_string());
                        }
                    }
                }
            }
            if let Some(partners) = row.get("partners").and_then(Value::as_array) {
                for partner in partners {
                    if let Some(id) = partner.get("id").and_then(Value::as_str) {
                        if let Some(aug_id) = id.strip_prefix("augment:") {
                            ids.push(aug_id.to_string());
                        }
                    }
                }
            }
            if let Some(components) = row.get("components").and_then(Value::as_array) {
                for comp in components {
                    if comp.get("kind").and_then(Value::as_str) == Some("augment") {
                        if let Some(id_val) = comp.get("id") {
                            let id_str = id_val
                                .as_str()
                                .map(ToString::to_string)
                                .or_else(|| id_val.as_i64().map(|n| n.to_string()));
                            if let Some(s) = id_str {
                                ids.push(s);
                            }
                        }
                    }
                }
            }
            ids
        })
        .collect::<Vec<_>>();
    let mut unique_augment_ids = augment_ids;
    unique_augment_ids.sort();
    unique_augment_ids.dedup();
    let augment_map = resolve_augments(Some(&Value::Array(
        unique_augment_ids.into_iter().map(Value::String).collect(),
    )))
    .await
    .into_iter()
    .map(|augment| (augment.id.clone(), augment))
    .collect::<HashMap<_, _>>();

    for row in rows.iter_mut() {
        let Some(object) = row.as_object_mut() else {
            continue;
        };
        let id = object.get("id").and_then(Value::as_str).unwrap_or_default();
        if let Some(champion_id) = id
            .strip_prefix("champion:")
            .and_then(|id| id.parse::<i64>().ok())
        {
            if let Some(champion) = champions_by_id.get(&champion_id) {
                object.insert("name".to_string(), Value::String(champion.name.clone()));
                object.insert(
                    "icon_url".to_string(),
                    Value::String(champion.square_icon_url.clone()),
                );
            }
        } else if let Some(item_id) = id
            .strip_prefix("item:")
            .and_then(|id| id.parse::<i32>().ok())
        {
            if let Some(item_info) = items.get(&item_id) {
                object.insert("name".to_string(), Value::String(item_info.name.clone()));
                object.insert(
                    "icon_url".to_string(),
                    Value::String(item_info.icon_url.clone()),
                );
                if let Some(item_class) = &item_info.item_class {
                    object.insert("item_class".to_string(), Value::String(item_class.clone()));
                }
                if let Some(desc) = &item_info.description {
                    object.insert("description".to_string(), Value::String(desc.clone()));
                }
                object.insert(
                    "price_total".to_string(),
                    Value::Number(item_info.price_total.into()),
                );
                object.insert(
                    "categories".to_string(),
                    serde_json::to_value(&item_info.categories).unwrap_or_else(|_| json!([])),
                );
            }
        } else if let Some(augment_id) = id.strip_prefix("augment:") {
            if let Some(augment) = augment_map.get(augment_id) {
                object.insert("name".to_string(), Value::String(augment.name.clone()));
                object.insert(
                    "icon_url".to_string(),
                    Value::String(augment.icon_url.clone()),
                );
                if let Some(rarity) = &augment.rarity {
                    object.insert("rarity".to_string(), Value::String(rarity.clone()));
                }
                if let Some(tooltip) = &augment.tooltip {
                    object.insert("description".to_string(), Value::String(tooltip.clone()));
                }
            }
        } else if id.starts_with("combo:") && object.get("components").is_none() {
            // Reconstruct components if missing
            let reconstructed = id
                .trim_start_matches("combo:")
                .split('+')
                .map(|part| {
                    if let Some(cid) = part.strip_prefix("champion:") {
                        json!({ "kind": "champion", "id": cid, "name": format!("Champion #{cid}"), "icon_url": "" })
                    } else if let Some(iid) = part.strip_prefix("item:") {
                        json!({ "kind": "item", "id": iid, "name": format!("Item #{iid}"), "icon_url": "" })
                    } else if let Some(aid) = part.strip_prefix("augment:") {
                        json!({ "kind": "augment", "id": aid, "name": format!("Augment #{aid}"), "icon_url": "" })
                    } else {
                        json!({ "kind": "unknown", "id": part, "name": part, "icon_url": "" })
                    }
                })
                .collect::<Vec<_>>();
            object.insert("components".to_string(), Value::Array(reconstructed));
        }

        if let Some(components) = object.get_mut("components").and_then(Value::as_array_mut) {
            let mut component_names = Vec::new();
            for component in components {
                let Some(comp_obj) = component.as_object_mut() else {
                    continue;
                };
                let comp_kind = comp_obj
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let comp_id_str = comp_obj
                    .get("id")
                    .and_then(|v| {
                        v.as_str()
                            .map(ToString::to_string)
                            .or_else(|| v.as_i64().map(|n| n.to_string()))
                    })
                    .unwrap_or_default();

                match comp_kind {
                    "champion" => {
                        if let Ok(champ_id) = comp_id_str.parse::<i64>() {
                            if let Some(champion) = champions_by_id.get(&champ_id) {
                                comp_obj.insert(
                                    "name".to_string(),
                                    Value::String(champion.name.clone()),
                                );
                                comp_obj.insert(
                                    "icon_url".to_string(),
                                    Value::String(champion.square_icon_url.clone()),
                                );
                                component_names.push(champion.name.clone());
                            } else {
                                component_names.push(
                                    comp_obj
                                        .get("name")
                                        .and_then(Value::as_str)
                                        .unwrap_or("Champion")
                                        .to_string(),
                                );
                            }
                        }
                    }
                    "item" => {
                        if let Ok(item_id) = comp_id_str.parse::<i32>() {
                            if let Some(item_info) = items.get(&item_id) {
                                comp_obj.insert(
                                    "name".to_string(),
                                    Value::String(item_info.name.clone()),
                                );
                                comp_obj.insert(
                                    "icon_url".to_string(),
                                    Value::String(item_info.icon_url.clone()),
                                );
                                if let Some(item_class) = &item_info.item_class {
                                    comp_obj.insert(
                                        "item_class".to_string(),
                                        Value::String(item_class.clone()),
                                    );
                                }
                                if let Some(desc) = &item_info.description {
                                    comp_obj.insert(
                                        "description".to_string(),
                                        Value::String(desc.clone()),
                                    );
                                }
                                comp_obj.insert(
                                    "price_total".to_string(),
                                    Value::Number(item_info.price_total.into()),
                                );
                                comp_obj.insert(
                                    "categories".to_string(),
                                    serde_json::to_value(&item_info.categories)
                                        .unwrap_or_else(|_| json!([])),
                                );
                                component_names.push(item_info.name.clone());
                            } else {
                                component_names.push(
                                    comp_obj
                                        .get("name")
                                        .and_then(Value::as_str)
                                        .unwrap_or("Item")
                                        .to_string(),
                                );
                            }
                        }
                    }
                    "augment" => {
                        if let Some(augment) = augment_map.get(&comp_id_str) {
                            comp_obj
                                .insert("name".to_string(), Value::String(augment.name.clone()));
                            comp_obj.insert(
                                "icon_url".to_string(),
                                Value::String(augment.icon_url.clone()),
                            );
                            if let Some(rarity) = &augment.rarity {
                                comp_obj
                                    .insert("rarity".to_string(), Value::String(rarity.clone()));
                            }
                            if let Some(tooltip) = &augment.tooltip {
                                comp_obj.insert(
                                    "description".to_string(),
                                    Value::String(tooltip.clone()),
                                );
                            }
                            component_names.push(augment.name.clone());
                        } else {
                            component_names.push(
                                comp_obj
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Augment")
                                    .to_string(),
                            );
                        }
                    }
                    _ => {}
                }
            }
            if !component_names.is_empty() {
                object.insert(
                    "name".to_string(),
                    Value::String(component_names.join(" + ")),
                );
            }
        }
        if let Some(contexts) = object.get_mut("champions").and_then(Value::as_array_mut) {
            for context in contexts {
                let Some(context_object) = context.as_object_mut() else {
                    continue;
                };
                if let Some(champion_id) = context_object.get("id").and_then(Value::as_i64) {
                    if let Some(champion) = champions_by_id.get(&champion_id) {
                        context_object
                            .insert("name".to_string(), Value::String(champion.name.clone()));
                        context_object.insert(
                            "icon_url".to_string(),
                            Value::String(champion.square_icon_url.clone()),
                        );
                    }
                }
            }
        }
        if let Some(contexts) = object.get_mut("partners").and_then(Value::as_array_mut) {
            for context in contexts {
                let Some(context_object) = context.as_object_mut() else {
                    continue;
                };
                let partner_id = context_object
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if let Some(item_id) = partner_id
                    .strip_prefix("item:")
                    .and_then(|id| id.parse::<i32>().ok())
                {
                    if let Some(item_info) = items.get(&item_id) {
                        context_object
                            .insert("name".to_string(), Value::String(item_info.name.clone()));
                        context_object.insert(
                            "icon_url".to_string(),
                            Value::String(item_info.icon_url.clone()),
                        );
                        if let Some(item_class) = &item_info.item_class {
                            context_object.insert(
                                "item_class".to_string(),
                                Value::String(item_class.clone()),
                            );
                        }
                        if let Some(desc) = &item_info.description {
                            context_object
                                .insert("description".to_string(), Value::String(desc.clone()));
                        }
                        context_object.insert(
                            "price_total".to_string(),
                            Value::Number(item_info.price_total.into()),
                        );
                    }
                } else if let Some(augment_id) = partner_id.strip_prefix("augment:") {
                    if let Some(augment) = augment_map.get(augment_id) {
                        context_object
                            .insert("name".to_string(), Value::String(augment.name.clone()));
                        context_object.insert(
                            "icon_url".to_string(),
                            Value::String(augment.icon_url.clone()),
                        );
                        if let Some(rarity) = &augment.rarity {
                            context_object
                                .insert("rarity".to_string(), Value::String(rarity.clone()));
                        }
                        if let Some(tooltip) = &augment.tooltip {
                            context_object
                                .insert("description".to_string(), Value::String(tooltip.clone()));
                        }
                    }
                }
            }
        }
    }

    // Filter out any combo rows containing boots or starters
    rows.retain(|row| {
        let is_combo = row
            .get("id")
            .and_then(Value::as_str)
            .map_or(false, |id| id.starts_with("combo:"));
        if !is_combo {
            return true;
        }
        if let Some(components) = row.get("components").and_then(Value::as_array) {
            for comp in components {
                if comp.get("kind").and_then(Value::as_str) == Some("item") {
                    if let Some(id_val) = comp.get("id") {
                        let id = id_val
                            .as_i64()
                            .or_else(|| id_val.as_str().and_then(|s| s.parse::<i64>().ok()));
                        if let Some(id) = id {
                            if crate::agent::is_boot_item_id(id)
                                || crate::agent::is_starter_item_id(id)
                            {
                                return false;
                            }
                        }
                    }
                    if comp.get("item_class").and_then(Value::as_str) == Some("boot") {
                        return false;
                    }
                    if let Some(name) = comp.get("name").and_then(Value::as_str) {
                        let low = name.to_lowercase();
                        if low.contains("boot")
                            || low.contains("greaves")
                            || low.contains("shoes")
                            || low.contains("steelcaps")
                            || low.contains("treads")
                        {
                            return false;
                        }
                    }
                }
            }
        }
        true
    });

    Ok(response)
}

struct ParticipantFact {
    win: bool,
    item_ids: Vec<i64>,
    augment_ids: Vec<String>,
    spell1_id: u64,
    spell2_id: u64,
    damage: i64,
    damage_taken: i64,
    true_damage: i64,
    damage_mitigated: i64,
    total_heal: i64,
    turret_damage: i64,
    game_duration: i64,
}

#[derive(Default, Clone)]
struct MetricAggregate {
    games: i64,
    wins: i64,
    damage: i64,
    damage_taken: i64,
    true_damage: i64,
    damage_mitigated: i64,
    total_heal: i64,
    turret_damage: i64,
    dpm_sum: f64,
}

fn score_metric(agg: &MetricAggregate, strategy_id: &str, base_wr: f64) -> f64 {
    let games = agg.games.max(1) as f64;
    let m = 10.0;
    match strategy_id {
        "win_rate" => (agg.wins as f64 + m * base_wr) / (games + m),
        "damage_dealt" => (agg.damage as f64 + m * 20000.0) / (games + m),
        "damage_per_minute" => (agg.dpm_sum + m * 1500.0) / (games + m),
        "true_damage" => (agg.true_damage as f64 + m * 2000.0) / (games + m),
        "damage_taken" => (agg.damage_taken as f64 + m * 25000.0) / (games + m),
        "damage_mitigated" => (agg.damage_mitigated as f64 + m * 20000.0) / (games + m),
        "healing" => (agg.total_heal as f64 + m * 8000.0) / (games + m),
        "turret_damage" => (agg.turret_damage as f64 + m * 3000.0) / (games + m),
        _ => (agg.wins as f64 + m * base_wr) / (games + m),
    }
}

fn is_excluded_mayhem_item(id: i64, item_info: Option<&CachedItemInfo>) -> bool {
    if id == 0 || id == 2050 || id == 2052 || id == 220013 {
        return true;
    }
    if let Some(info) = item_info {
        if info.price_total == 6000 {
            return true;
        }
        let lower = info.name.to_lowercase();
        if lower.contains("spatula")
            || lower.contains("ultra hydra")
            || lower.contains("wooglet")
            || lower.contains("void immolation")
            || lower.contains("adaptive helm")
            || lower.contains("poro-snax")
        {
            return true;
        }
        if info
            .categories
            .iter()
            .any(|c| c.eq_ignore_ascii_case("consumable") || c.eq_ignore_ascii_case("trinket"))
        {
            return true;
        }
    }
    false
}

fn is_starter_item(id: i64, item_info: Option<&CachedItemInfo>) -> bool {
    match id {
        3177 | 3184 | 2051 | 3112 | 3070 | 1054 | 1055 | 1056 | 1082 | 1083 => true,
        _ => {
            if let Some(info) = item_info {
                let lower = info.name.to_lowercase();
                lower.starts_with("guardian's ")
                    || lower.starts_with("doran's ")
                    || lower == "tear of the goddess"
            } else {
                false
            }
        }
    }
}

/// Identifies League of Legends item exclusivity groups (items that cannot be purchased together).
/// Reference: CDragon item stringtable groups (item_unique_hydra, item_unique_lifeline, item_unique_lastwhisper, etc.).
fn get_item_exclusive_group(id: i64, item_info: Option<&CachedItemInfo>) -> Option<&'static str> {
    if let Some(info) = item_info {
        let name_low = info.name.to_lowercase();
        let desc = info.description.as_deref().unwrap_or("");

        // 1. Hydra / Cleave items (e.g. Ravenous Hydra, Titanic Hydra, Profane Hydra, Stridebreaker, Tiamat)
        if name_low.contains("hydra")
            || name_low.contains("tiamat")
            || name_low.contains("stridebreaker")
            || desc.contains("<passive>Cleave</passive>")
            || matches!(
                id,
                3074 | 3077
                    | 3748
                    | 6631
                    | 6698
                    | 223074
                    | 223748
                    | 226631
                    | 226668
                    | 226698
                    | 773074
                    | 773077
            )
        {
            return Some("hydra");
        }

        // 2. Lifeline items (Sterak's Gage, Maw of Malmortius, Immortal Shieldbow, Seraph's Embrace, Hexdrinker)
        if name_low.contains("sterak")
            || name_low.contains("malmortius")
            || name_low.contains("shieldbow")
            || name_low.contains("hexdrinker")
            || desc.contains("<passive>Lifeline</passive>")
            || matches!(
                id,
                3053 | 3155 | 3156 | 6673 | 223053 | 223156 | 226673 | 773155 | 773156
            )
        {
            return Some("lifeline");
        }

        // 3. Percent Armor Penetration / Last Whisper items (Lord Dominik's, Serylda's, Mortal Reminder, Terminus)
        if name_low.contains("last whisper")
            || name_low.contains("dominik")
            || name_low.contains("serylda")
            || name_low.contains("mortal reminder")
            || name_low.contains("terminus")
            || matches!(
                id,
                3033 | 3035 | 3036 | 3302 | 6694 | 223033 | 223036 | 223302 | 226694 | 773035
            )
        {
            return Some("last_whisper");
        }

        // 4. Crit Modifier / Marksman Capstone items (Infinity Edge, Navori Flickerblade)
        if name_low.contains("infinity edge")
            || name_low.contains("navori")
            || matches!(id, 3031 | 6675 | 223031 | 226675)
        {
            return Some("crit_modifier");
        }

        // 5. Spellblade items (Trinity Force, Lich Bane, Iceborn Gauntlet, Essence Reaver, Bloodsong, Sheen)
        if name_low.contains("trinity force")
            || name_low.contains("lich bane")
            || name_low.contains("iceborn")
            || name_low.contains("bloodsong")
            || desc.contains("<passive>Spellblade</passive>")
            || matches!(
                id,
                3057 | 3078
                    | 3100
                    | 3508
                    | 3869
                    | 6662
                    | 6901
                    | 223078
                    | 223100
                    | 223508
                    | 226662
                    | 773057
            )
        {
            return Some("spellblade");
        }

        // 6. Tear / Mana Charge items (Manamune, Muramana, Archangel's, Seraph's, Winter's Approach, Fimbulwinter)
        if name_low.contains("manamune")
            || name_low.contains("muramana")
            || name_low.contains("archangel")
            || name_low.contains("seraph")
            || name_low.contains("fimbulwinter")
            || name_low.contains("winter's approach")
            || name_low.contains("tear of the goddess")
            || desc.contains("<passive>Mana Charge</passive>")
            || matches!(
                id,
                3003 | 3004
                    | 3040
                    | 3042
                    | 3070
                    | 3119
                    | 3121
                    | 223003
                    | 223004
                    | 223040
                    | 223042
                    | 323040
            )
        {
            return Some("tear");
        }

        // 7. Immolate items (Sunfire Aegis, Hollow Radiance, Bami's Cinder)
        if name_low.contains("sunfire")
            || name_low.contains("hollow radiance")
            || name_low.contains("bami")
            || desc.contains("<passive>Immolate</passive>")
            || matches!(id, 3068 | 3751 | 6664 | 223068 | 226664)
        {
            return Some("immolate");
        }

        // 8. Glory items
        if matches!(id, 1082 | 3041) || name_low.contains("mejai") || name_low.contains("dark seal")
        {
            return Some("glory");
        }

        // 9. Boots (only 1 pair of boots can be bought)
        if info.item_class.as_deref() == Some("boot")
            || info
                .categories
                .iter()
                .any(|c| c.eq_ignore_ascii_case("boots"))
            || name_low.contains("greaves")
            || name_low.contains("shoes")
            || name_low.contains("steelcaps")
            || name_low.contains("treads")
            || matches!(
                id,
                1001 | 3005 | 3006 | 3009 | 3020 | 3047 | 3111 | 3117 | 3158
            )
        {
            return Some("boot");
        }
    }
    None
}

/// Computes slot-aware Bayesian-smoothed pick rate for items.
/// In League of Legends / ARAM Mayhem: 1 exclusive boot slot, 5 regular item slots.
fn calc_item_pick_rate(games: usize, total_games: usize, is_boot: bool) -> f64 {
    if total_games == 0 {
        return 0.0;
    }
    if is_boot {
        let p0 = 0.125; // 1 out of ~8 viable boots
        let m = 10.0;
        let pr = (games as f64 + m * p0) / (total_games as f64 + m);
        pr.clamp(0.0, 1.0)
    } else {
        let p0_slot = 0.007; // ~150 completed items
        let m_slots = 25.0; // ~5 games worth of 5 item slots
        let total_slots = (total_games * 5) as f64;
        let slot_share = (games as f64 + m_slots * p0_slot) / (total_slots + m_slots);
        (slot_share * 5.0).clamp(0.0, 1.0)
    }
}

pub fn get_situational_tag(id: i64, item_info: Option<&CachedItemInfo>) -> Option<&'static str> {
    match id {
        // Anti-Heal (Grievous Wounds)
        3033 | 6609 | 3165 | 3075 | 3916 | 3123 | 3076 => Some("Anti-Heal"),
        // Anti-Shield (Shield Reaver / Cursed Bloom)
        6695 | 4645 => Some("Anti-Shield"),
        // Armor Penetration
        3036 | 6694 | 3071 | 3302 => Some("Armor Penetration"),
        // Magic Penetration
        3135 | 3137 => Some("Magic Penetration"),
        // Anti-Magic / Spell Shield
        3156 | 6701 | 4401 | 3102 | 3814 => Some("Magic Resist"),
        // Armor / Anti-Burst / Stasis
        3157 | 6333 | 3110 | 3143 => Some("Armor / Anti-Burst"),
        // Cleanse / CC Immunity
        3139 | 6035 | 3140 => Some("Cleanse / QSS"),
        _ => {
            if let Some(info) = item_info {
                let lower = info.name.to_lowercase();
                let desc = info.description.as_deref().unwrap_or("").to_lowercase();
                if lower.contains("mortal reminder")
                    || lower.contains("chainsword")
                    || lower.contains("morellonomicon")
                    || lower.contains("thornmail")
                    || desc.contains("grievous wounds")
                {
                    return Some("Anti-Heal");
                }
                if lower.contains("serpent's fang")
                    || lower.contains("shadowflame")
                    || (desc.contains("shield")
                        && (desc.contains("destroy") || desc.contains("reduce")))
                {
                    return Some("Anti-Shield");
                }
                if lower.contains("dominik")
                    || lower.contains("serylda")
                    || lower.contains("cleaver")
                    || desc.contains("armor penetration")
                {
                    return Some("Armor Penetration");
                }
                if lower.contains("void staff")
                    || lower.contains("cryptbloom")
                    || desc.contains("magic penetration")
                {
                    return Some("Magic Penetration");
                }
                if lower.contains("kaenic")
                    || lower.contains("force of nature")
                    || lower.contains("malmortius")
                {
                    return Some("Magic Resist");
                }
                if lower.contains("zhonya")
                    || lower.contains("frozen heart")
                    || lower.contains("randuin")
                {
                    return Some("Armor / Anti-Burst");
                }
            }
            None
        }
    }
}

pub fn get_item_augment_warp_bonus(item_id: i64, augments: &[String]) -> f64 {
    let mut bonus = 0.0;
    for aug in augments {
        let low = aug.to_lowercase();
        if low == "marksmage" {
            if matches!(
                item_id,
                3115 | 3100 | 3089 | 3124 | 4645 | 3302 | 3091 | 3020
            ) {
                bonus += 3.0;
            }
        } else if low == "adapt" {
            if matches!(item_id, 3031 | 3072 | 3089 | 6333 | 6697 | 3074) {
                bonus += 2.5;
            }
        } else if low == "big shot" || low == "bigshot" {
            if matches!(item_id, 3046 | 3085 | 3031 | 6672 | 3094 | 3033 | 3006) {
                bonus += 2.5;
            }
        } else if low == "apex inventor" || low == "apexinventor" {
            if matches!(
                item_id,
                3084 | 6610 | 6665 | 6692 | 3157 | 6653 | 3074 | 3748
            ) {
                bonus += 2.5;
            }
        } else if low == "vulnerability" {
            if matches!(item_id, 3151 | 3097 | 3031 | 3068 | 6664) {
                bonus += 2.5;
            }
        } else if low == "goliath" {
            if matches!(item_id, 3084 | 3083 | 6667 | 3068 | 3005 | 3075 | 6701) {
                bonus += 2.5;
            }
        } else if low == "draw your sword" || low == "drawyoursword" {
            if matches!(item_id, 3074 | 3748 | 3078 | 3153 | 6333 | 3053) {
                bonus += 2.5;
            }
        }
    }
    bonus
}

pub fn build_augment_warped_archetype(
    augment: &str,
    champion_win_rate: f64,
) -> Option<MinedArchetype> {
    let lower = augment.to_lowercase();
    if lower == "marksmage" {
        Some(MinedArchetype {
            id: "augment_warp_marksmage".to_string(),
            label: "Augment Warp: AP On-Hit (Marksmage)".to_string(),
            core_items: vec![3115, 3100, 3089],
            boot_id: Some(3020),
            starter_id: Some(3184),
            continuation_items: vec![
                (3124, 100, 0.65, 0.45),
                (4645, 90, 0.64, 0.40),
                (3302, 80, 0.62, 0.35),
                (3091, 75, 0.61, 0.30),
                (4637, 70, 0.60, 0.28),
            ],
            situational_items: vec![
                (3165, 50, 0.60, 0.20),
                (4645, 60, 0.63, 0.25),
                (3135, 55, 0.62, 0.22),
                (3157, 65, 0.61, 0.24),
                (3102, 45, 0.59, 0.18),
            ],
            sample_games: 50,
            build_win_rate: (champion_win_rate + 0.08).clamp(0.5, 0.75),
            build_wpa: 0.08,
            spells: (4, 32),
            top_augments: vec![("Marksmage".to_string(), 50, 0.65, 0.10, 0.9)],
        })
    } else if lower == "adapt" {
        Some(MinedArchetype {
            id: "augment_warp_adapt".to_string(),
            label: "Augment Warp: AD-to-AP (ADAPt)".to_string(),
            core_items: vec![3031, 3072, 3089],
            boot_id: Some(3006),
            starter_id: Some(3177),
            continuation_items: vec![
                (6333, 90, 0.63, 0.40),
                (6697, 85, 0.62, 0.35),
                (3074, 80, 0.61, 0.32),
                (4645, 75, 0.60, 0.30),
            ],
            situational_items: vec![
                (3033, 50, 0.60, 0.20),
                (6695, 55, 0.61, 0.22),
                (3036, 60, 0.62, 0.25),
            ],
            sample_games: 40,
            build_win_rate: (champion_win_rate + 0.07).clamp(0.5, 0.75),
            build_wpa: 0.07,
            spells: (4, 32),
            top_augments: vec![("ADAPt".to_string(), 40, 0.64, 0.09, 0.9)],
        })
    } else if lower == "big shot" || lower == "bigshot" {
        Some(MinedArchetype {
            id: "augment_warp_bigshot".to_string(),
            label: "Augment Warp: AS to Crit (Big Shot)".to_string(),
            core_items: vec![3046, 3085, 3031],
            boot_id: Some(3006),
            starter_id: Some(3177),
            continuation_items: vec![
                (6672, 90, 0.64, 0.42),
                (3094, 85, 0.62, 0.38),
                (3033, 75, 0.61, 0.30),
                (3302, 70, 0.60, 0.28),
            ],
            situational_items: vec![
                (3033, 50, 0.60, 0.20),
                (6695, 55, 0.61, 0.22),
                (3036, 60, 0.62, 0.25),
            ],
            sample_games: 40,
            build_win_rate: (champion_win_rate + 0.07).clamp(0.5, 0.75),
            build_wpa: 0.07,
            spells: (4, 32),
            top_augments: vec![("Big Shot".to_string(), 40, 0.63, 0.08, 0.9)],
        })
    } else if lower == "apex inventor" || lower == "apexinventor" {
        Some(MinedArchetype {
            id: "augment_warp_apex_inventor".to_string(),
            label: "Augment Warp: Item Haste Engine (Apex Inventor)".to_string(),
            core_items: vec![3084, 6610, 6665],
            boot_id: Some(3047),
            starter_id: Some(2051),
            continuation_items: vec![
                (6692, 90, 0.63, 0.40),
                (3157, 80, 0.62, 0.35),
                (3074, 75, 0.61, 0.30),
            ],
            situational_items: vec![(3075, 50, 0.60, 0.20), (6701, 55, 0.61, 0.22)],
            sample_games: 40,
            build_win_rate: (champion_win_rate + 0.06).clamp(0.5, 0.75),
            build_wpa: 0.06,
            spells: (4, 32),
            top_augments: vec![("Apex Inventor".to_string(), 40, 0.62, 0.07, 0.9)],
        })
    } else {
        None
    }
}

pub fn build_stage_pathing(
    starter_item: &Option<MayhemBuildItem>,
    items: &[MayhemBuildItem],
    situational_items: &[MayhemBuildItem],
) -> Vec<MayhemStageItem> {
    let mut stages = Vec::new();

    // Stage 1: Starter + Boots (0-5 min / Lv 3-6)
    let mut stage1_items = Vec::new();
    if let Some(starter) = starter_item {
        stage1_items.push(starter.clone());
    }
    for it in items {
        if it.item_class.as_deref() == Some("boot")
            || it.name.to_lowercase().contains("boots")
            || it.name.to_lowercase().contains("greaves")
            || it.name.to_lowercase().contains("shoes")
            || it.name.to_lowercase().contains("treads")
        {
            stage1_items.push(it.clone());
            break;
        }
    }
    if stage1_items.is_empty() && !items.is_empty() {
        stage1_items.push(items[0].clone());
    }
    stages.push(MayhemStageItem {
        stage: 1,
        title: "Stage 1: Starter & Boots".to_string(),
        timing: "0 - 5 min (Lv 3 - 6)".to_string(),
        items: stage1_items,
    });

    let non_boots: Vec<MayhemBuildItem> = items
        .iter()
        .filter(|it| {
            it.item_class.as_deref() != Some("boot")
                && !it.name.to_lowercase().contains("boots")
                && !it.name.to_lowercase().contains("greaves")
                && !it.name.to_lowercase().contains("shoes")
                && !it.name.to_lowercase().contains("treads")
        })
        .cloned()
        .collect();

    // Stage 2: First Mythic/Core spike (Lv 7 augment synergy)
    let stage2_items: Vec<MayhemBuildItem> = non_boots.iter().take(2).cloned().collect();
    stages.push(MayhemStageItem {
        stage: 2,
        title: "Stage 2: First Core Spike".to_string(),
        timing: "5 - 10 min (Lv 7)".to_string(),
        items: stage2_items,
    });

    // Stage 3: Mid-game counter & synergy continuation (Lv 11 augment)
    let stage3_items: Vec<MayhemBuildItem> = non_boots.iter().skip(2).take(2).cloned().collect();
    stages.push(MayhemStageItem {
        stage: 3,
        title: "Stage 3: Synergy Continuation".to_string(),
        timing: "10 - 15 min (Lv 11)".to_string(),
        items: stage3_items,
    });

    // Stage 4: Capstone & Situational (Lv 15+)
    let mut stage4_items: Vec<MayhemBuildItem> =
        non_boots.iter().skip(4).take(2).cloned().collect();
    for sit in situational_items.iter().take(2) {
        if !stage4_items.iter().any(|x| x.id == sit.id) {
            stage4_items.push(sit.clone());
        }
    }
    stages.push(MayhemStageItem {
        stage: 4,
        title: "Stage 4: Capstone & Situational".to_string(),
        timing: "15+ min (Lv 15+)".to_string(),
        items: stage4_items,
    });

    stages
}

pub fn compute_live_build_advice(
    strategy: &MayhemBuildStrategy,
    current_gold: f64,
    inventory_item_ids: &[i64],
    level: u32,
    _game_time: f64,
) -> MayhemLiveAdvice {
    let owned_ids: HashSet<i64> = inventory_item_ids.iter().copied().collect();

    let current_stage = if level >= 15 || inventory_item_ids.len() >= 5 {
        4
    } else if level >= 11 || inventory_item_ids.len() >= 3 {
        3
    } else if level >= 7 || inventory_item_ids.len() >= 2 {
        2
    } else {
        1
    };

    let stage_title = match current_stage {
        1 => "Stage 1: Starter & Boots (Lv 3-6)",
        2 => "Stage 2: First Core Spike (Lv 7)",
        3 => "Stage 3: Synergy Continuation (Lv 11)",
        _ => "Stage 4: Capstone & Situational (Lv 15+)",
    }
    .to_string();

    let next_target = strategy
        .items
        .iter()
        .find(|it| !owned_ids.contains(&it.id))
        .cloned();

    let gold_needed = if let Some(ref target) = next_target {
        (target.price_total as f64 - current_gold).max(0.0) as i64
    } else {
        0
    };

    let mut immediate_purchases = Vec::new();
    let mut seen_ids = HashSet::new();

    if let Some(ref target) = next_target {
        if (target.price_total as f64) <= current_gold {
            immediate_purchases.push(target.clone());
            seen_ids.insert(target.id);
        }
    }

    for it in &strategy.items {
        if !owned_ids.contains(&it.id)
            && !seen_ids.contains(&it.id)
            && (it.price_total as f64) <= current_gold
        {
            immediate_purchases.push(it.clone());
            seen_ids.insert(it.id);
        }
    }

    for it in &strategy.situational_items {
        if !owned_ids.contains(&it.id)
            && !seen_ids.contains(&it.id)
            && (it.price_total as f64) <= current_gold
        {
            immediate_purchases.push(it.clone());
            seen_ids.insert(it.id);
        }
    }

    for it in &strategy.alternative_items {
        if immediate_purchases.len() >= 6 {
            break;
        }
        if !owned_ids.contains(&it.id)
            && !seen_ids.contains(&it.id)
            && (it.price_total as f64) <= current_gold
        {
            immediate_purchases.push(it.clone());
            seen_ids.insert(it.id);
        }
    }

    MayhemLiveAdvice {
        current_stage,
        stage_title,
        immediate_purchases,
        next_target,
        gold_needed,
    }
}

pub struct MinedArchetype {
    pub id: String,
    pub label: String,
    pub core_items: Vec<i64>,
    pub boot_id: Option<i64>,
    pub starter_id: Option<i64>,
    pub continuation_items: Vec<(i64, usize, f64, f64)>,
    pub situational_items: Vec<(i64, usize, f64, f64)>,
    pub sample_games: usize,
    pub build_win_rate: f64,
    pub build_wpa: f64,
    pub spells: (u64, u64),
    pub top_augments: Vec<(String, usize, f64, f64, f64)>,
}

fn classify_archetype_prefix(
    core_items: &[i64],
    items_catalog: &HashMap<i32, CachedItemInfo>,
) -> &'static str {
    let mut crit_count = 0;
    let mut ap_count = 0;
    let mut tank_count = 0;
    let mut lethality_count = 0;

    for &id in core_items {
        if let Some(info) = items_catalog.get(&(id as i32)) {
            let desc = info.description.as_deref().unwrap_or("").to_lowercase();
            let name = info.name.to_lowercase();
            if desc.contains("critical strike")
                || desc.contains("crit")
                || name.contains("infinity")
                || name.contains("collector")
                || name.contains("wildarrows")
            {
                crit_count += 1;
            }
            if desc.contains("ability power")
                || desc.contains("<ap>")
                || desc.contains("magic damage")
                || name.contains("rabadon")
                || name.contains("liandry")
                || name.contains("luden")
            {
                ap_count += 1;
            }
            if desc.contains("armor")
                || desc.contains("magic resist")
                || desc.contains("health")
                || name.contains("heartsteel")
                || name.contains("warmog")
                || name.contains("jak'sho")
            {
                tank_count += 1;
            }
            if desc.contains("lethality")
                || desc.contains("armor penetration")
                || name.contains("hubris")
                || name.contains("opportunity")
                || name.contains("ghostblade")
            {
                lethality_count += 1;
            }
        }
    }

    if crit_count >= 2 {
        "Crit"
    } else if lethality_count >= 2 {
        "Lethality"
    } else if ap_count >= 2 {
        "AP"
    } else if tank_count >= 2 {
        "Tank"
    } else if crit_count == 1 && lethality_count == 1 {
        "Crit/Lethality"
    } else {
        "Core"
    }
}

fn generate_archetype_label(
    core_items: &[i64],
    items_catalog: &HashMap<i32, CachedItemInfo>,
) -> String {
    if core_items.is_empty() {
        return "Core Build".to_string();
    }
    let names: Vec<String> = core_items
        .iter()
        .take(2)
        .map(|id| {
            if let Some(info) = items_catalog.get(&(*id as i32)) {
                let name = &info.name;
                if name.starts_with("Blade of the Ruined King") {
                    "BotRK".to_string()
                } else if name.starts_with("Lord Dominik's") {
                    "LDR".to_string()
                } else if name.starts_with("Infinity Edge") {
                    "IE".to_string()
                } else if name.starts_with("Essence Reaver") {
                    "ER".to_string()
                } else if name.starts_with("Titanic Hydra") {
                    "Titanic".to_string()
                } else if name.starts_with("Ravenous Hydra") {
                    "Ravenous".to_string()
                } else if name.starts_with("Immortal Shieldbow") {
                    "Shieldbow".to_string()
                } else {
                    name.split_whitespace()
                        .take(2)
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            } else {
                format!("Item #{id}")
            }
        })
        .collect();

    names.join(" + ")
}

// ponytail: true item purchase order is not recorded in ARAM Mayhem match history timelines; slot index (0..5) acts as order proxy.
fn mine_build_archetypes(
    facts: &[&ParticipantFact],
    items_catalog: &HashMap<i32, CachedItemInfo>,
    champion_win_rate: f64,
) -> Vec<MinedArchetype> {
    if facts.is_empty() {
        return Vec::new();
    }

    struct ParsedFact {
        legendaries: Vec<(i64, usize)>,
        boot: Option<i64>,
        starter: Option<i64>,
        win: bool,
        spell1: u64,
        spell2: u64,
        augments: Vec<String>,
    }

    let parsed: Vec<ParsedFact> = facts
        .iter()
        .map(|f| {
            let mut legendaries = Vec::new();
            let mut boot = None;
            let mut starter = None;
            for (slot, &it) in f.item_ids.iter().enumerate() {
                let info = items_catalog.get(&(it as i32));
                if is_excluded_mayhem_item(it, info) {
                    continue;
                }
                if is_starter_item(it, info) {
                    if starter.is_none() {
                        starter = Some(it);
                    }
                    continue;
                }
                let is_boot = info
                    .map(|i| {
                        i.item_class.as_deref() == Some("boot")
                            || i.categories.iter().any(|c| c.eq_ignore_ascii_case("boots"))
                    })
                    .unwrap_or(false);
                if is_boot {
                    if boot.is_none() {
                        boot = Some(it);
                    }
                    continue;
                }
                if info
                    .map(|i| i.price_total >= 1800 || i.item_class.as_deref() == Some("legendary"))
                    .unwrap_or(true)
                {
                    legendaries.push((it, slot));
                }
            }
            ParsedFact {
                legendaries,
                boot,
                starter,
                win: f.win,
                spell1: f.spell1_id,
                spell2: f.spell2_id,
                augments: f.augment_ids.clone(),
            }
        })
        .collect();

    let mut triple_stats: HashMap<(i64, i64, i64), (usize, usize, usize, usize, usize)> =
        HashMap::new();
    let mut pair_stats: HashMap<(i64, i64), (usize, usize, usize, usize)> = HashMap::new();

    for p in &parsed {
        let n = p.legendaries.len();
        if n >= 3 {
            for i in 0..n {
                for j in (i + 1)..n {
                    for k in (j + 1)..n {
                        let (it1, s1) = p.legendaries[i];
                        let (it2, s2) = p.legendaries[j];
                        let (it3, s3) = p.legendaries[k];

                        let g1 = get_item_exclusive_group(it1, items_catalog.get(&(it1 as i32)));
                        let g2 = get_item_exclusive_group(it2, items_catalog.get(&(it2 as i32)));
                        let g3 = get_item_exclusive_group(it3, items_catalog.get(&(it3 as i32)));
                        if (g1.is_some() && (g1 == g2 || g1 == g3)) || (g2.is_some() && g2 == g3) {
                            continue;
                        }

                        let mut sorted = [(it1, s1), (it2, s2), (it3, s3)];
                        sorted.sort_by_key(|x| x.0);
                        let key = (sorted[0].0, sorted[1].0, sorted[2].0);
                        let entry = triple_stats.entry(key).or_insert((0, 0, 0, 0, 0));
                        entry.0 += 1;
                        if p.win {
                            entry.1 += 1;
                        }
                        entry.2 += sorted[0].1;
                        entry.3 += sorted[1].1;
                        entry.4 += sorted[2].1;
                    }
                }
            }
        }
        if n >= 2 {
            for i in 0..n {
                for j in (i + 1)..n {
                    let (it1, s1) = p.legendaries[i];
                    let (it2, s2) = p.legendaries[j];
                    let g1 = get_item_exclusive_group(it1, items_catalog.get(&(it1 as i32)));
                    let g2 = get_item_exclusive_group(it2, items_catalog.get(&(it2 as i32)));
                    if g1.is_some() && g1 == g2 {
                        continue;
                    }
                    let mut sorted = [(it1, s1), (it2, s2)];
                    sorted.sort_by_key(|x| x.0);
                    let key = (sorted[0].0, sorted[1].0);
                    let entry = pair_stats.entry(key).or_insert((0, 0, 0, 0));
                    entry.0 += 1;
                    if p.win {
                        entry.1 += 1;
                    }
                    entry.2 += sorted[0].1;
                    entry.3 += sorted[1].1;
                }
            }
        }
    }

    let min_support = if facts.len() <= 15 {
        1
    } else {
        ((facts.len() as f64 * 0.015).ceil() as usize).clamp(3, 30)
    };

    struct ScoredCore {
        items: Vec<i64>,
        slot_sums: Vec<usize>,
        games: usize,
        wins: usize,
        score: f64,
    }

    let mut scored_triples: Vec<ScoredCore> = triple_stats
        .into_iter()
        .filter(|(_, (g, _, _, _, _))| *g >= min_support)
        .map(|(k, (g, w, s1, s2, s3))| {
            let m = 20.0;
            let wr = (w as f64 + m * champion_win_rate) / (g as f64 + m);
            let pick_rate = g as f64 / facts.len().max(1) as f64;
            let score = (wr - champion_win_rate) + 0.12 * (pick_rate * 100.0).sqrt() / 10.0;
            ScoredCore {
                items: vec![k.0, k.1, k.2],
                slot_sums: vec![s1, s2, s3],
                games: g,
                wins: w,
                score,
            }
        })
        .collect();

    scored_triples.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut selected_cores: Vec<ScoredCore> = Vec::new();
    for cand in scored_triples {
        let cand_set: HashSet<i64> = cand.items.iter().copied().collect();
        let overlap = selected_cores.iter().any(|existing| {
            let shared = existing
                .items
                .iter()
                .filter(|x| cand_set.contains(x))
                .count();
            shared >= 2
        });
        if !overlap {
            selected_cores.push(cand);
            if selected_cores.len() >= 3 {
                break;
            }
        }
    }

    if selected_cores.is_empty() {
        let min_pair_supp = if facts.len() <= 15 { 1 } else { 2 };
        let mut scored_pairs: Vec<ScoredCore> = pair_stats
            .into_iter()
            .filter(|(_, (g, _, _, _))| *g >= min_pair_supp)
            .map(|(k, (g, w, s1, s2))| {
                let m = 15.0;
                let wr = (w as f64 + m * champion_win_rate) / (g as f64 + m);
                let pick_rate = g as f64 / facts.len().max(1) as f64;
                let score = (wr - champion_win_rate) + 0.10 * (pick_rate * 100.0).sqrt() / 10.0;
                ScoredCore {
                    items: vec![k.0, k.1],
                    slot_sums: vec![s1, s2],
                    games: g,
                    wins: w,
                    score,
                }
            })
            .collect();
        scored_pairs.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for cand in scored_pairs {
            let cand_set: HashSet<i64> = cand.items.iter().copied().collect();
            let overlap = selected_cores.iter().any(|existing| {
                let shared = existing
                    .items
                    .iter()
                    .filter(|x| cand_set.contains(x))
                    .count();
                shared >= 1
            });
            if !overlap {
                selected_cores.push(cand);
                if selected_cores.len() >= 3 {
                    break;
                }
            }
        }
    }

    let mut archetypes = Vec::new();

    for (core_idx, core) in selected_cores.into_iter().enumerate() {
        let mut items_with_slots: Vec<(i64, f64)> = core
            .items
            .iter()
            .zip(core.slot_sums.iter())
            .map(|(&id, &sum)| (id, sum as f64 / core.games.max(1) as f64))
            .collect();
        items_with_slots.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let ordered_core: Vec<i64> = items_with_slots.iter().map(|x| x.0).collect();

        let sub_facts: Vec<&ParsedFact> = parsed
            .iter()
            .filter(|p| {
                let p_items: HashSet<i64> = p.legendaries.iter().map(|x| x.0).collect();
                ordered_core.iter().all(|c| p_items.contains(c))
            })
            .collect();

        let sample_games = sub_facts.len().max(core.games);
        let sample_wins = if sub_facts.is_empty() {
            core.wins
        } else {
            sub_facts.iter().filter(|p| p.win).count()
        };
        let build_win_rate = sample_wins as f64 / sample_games.max(1) as f64;
        let build_wpa = build_win_rate - champion_win_rate;

        let mut boot_counts: HashMap<i64, usize> = HashMap::new();
        for p in &sub_facts {
            if let Some(b) = p.boot {
                *boot_counts.entry(b).or_default() += 1;
            }
        }
        let boot_id = boot_counts.into_iter().max_by_key(|x| x.1).map(|x| x.0);

        let mut starter_counts: HashMap<i64, usize> = HashMap::new();
        for p in &sub_facts {
            if let Some(s) = p.starter {
                *starter_counts.entry(s).or_default() += 1;
            }
        }
        let starter_id = starter_counts
            .into_iter()
            .max_by_key(|x| x.1)
            .and_then(|(s, count)| {
                if count >= 2 && (count as f64 / sample_games as f64) >= 0.15 {
                    Some(s)
                } else {
                    None
                }
            });

        let mut item_sub_counts: HashMap<i64, (usize, usize)> = HashMap::new();
        for p in &sub_facts {
            for &(it, _) in &p.legendaries {
                if !ordered_core.contains(&it) {
                    let e = item_sub_counts.entry(it).or_insert((0, 0));
                    e.0 += 1;
                    if p.win {
                        e.1 += 1;
                    }
                }
            }
        }

        let mut continuation_items = Vec::new();
        let mut situational_items = Vec::new();

        for (it, (g, w)) in item_sub_counts {
            let cond_pr = g as f64 / sample_games.max(1) as f64;
            let cond_wr = w as f64 / g.max(1) as f64;
            if cond_pr >= 0.08 && g >= 2 {
                continuation_items.push((it, g, cond_wr, cond_pr));
            } else if g >= 1 {
                situational_items.push((it, g, cond_wr, cond_pr));
            }
        }

        continuation_items.sort_by(|a, b| {
            let sa = (a.2 - champion_win_rate) * 0.4 + a.3 * 0.6;
            let sb = (b.2 - champion_win_rate) * 0.4 + b.3 * 0.6;
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        situational_items.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut spell_pairs: HashMap<(u64, u64), usize> = HashMap::new();
        for p in &sub_facts {
            if p.spell1 > 0 && p.spell2 > 0 {
                let pair = (p.spell1.min(p.spell2), p.spell1.max(p.spell2));
                *spell_pairs.entry(pair).or_default() += 1;
            }
        }
        let top_spells = spell_pairs
            .into_iter()
            .max_by_key(|x| x.1)
            .map(|x| x.0)
            .unwrap_or((4, 32));

        // Augments - mine most common augments associated with this champion and core items
        let mut aug_counts: HashMap<String, (usize, usize)> = HashMap::new();
        for p in &sub_facts {
            for a in &p.augments {
                if !a.is_empty() && a != "0" {
                    let entry = aug_counts.entry(a.clone()).or_insert((0, 0));
                    entry.0 += 1;
                    if p.win {
                        entry.1 += 1;
                    }
                }
            }
        }
        let mut sorted_augs: Vec<(String, usize, usize)> = aug_counts
            .into_iter()
            .map(|(id, (g, w))| (id, g, w))
            .collect();
        sorted_augs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2)));

        let sample_g = sample_games.max(1) as f64;
        let top_augments: Vec<(String, usize, f64, f64, f64)> = sorted_augs
            .into_iter()
            .take(12)
            .map(|(id, g, w)| {
                let m = 3.0;
                let wr = (w as f64 + m * build_win_rate) / (g as f64 + m);
                let wpa = wr - champion_win_rate;
                let pr = g as f64 / sample_g;
                (id, g, wr, wpa, pr)
            })
            .collect();

        let prefix = classify_archetype_prefix(&ordered_core, items_catalog);
        let names = generate_archetype_label(&ordered_core, items_catalog);
        let label = format!("{prefix}: {names}");

        archetypes.push(MinedArchetype {
            id: format!("archetype_{}", core_idx + 1),
            label,
            core_items: ordered_core,
            boot_id,
            starter_id,
            continuation_items,
            situational_items,
            sample_games,
            build_win_rate,
            build_wpa,
            spells: top_spells,
            top_augments,
        });
    }

    archetypes
}

async fn mayhem_item(
    id: i64,
    catalog: &HashMap<i32, CachedItemInfo>,
    agg: Option<&MetricAggregate>,
    champion_win_rate: f64,
    total_games: usize,
) -> MayhemBuildItem {
    let info = catalog.get(&(id as i32));
    let is_boot = info
        .map(|i| {
            i.item_class.as_deref() == Some("boot")
                || i.categories.iter().any(|c| c.eq_ignore_ascii_case("boots"))
        })
        .unwrap_or(false);
    let raw_icon = info.map(|i| i.icon_url.clone()).unwrap_or_default();
    let (games, wins) = agg
        .map(|a| (a.games as usize, a.wins as f64))
        .unwrap_or((0, 0.0));
    let win_rate = if games > 0 {
        wins / games as f64
    } else {
        champion_win_rate
    };
    MayhemBuildItem {
        id,
        name: info
            .map(|i| i.name.clone())
            .unwrap_or_else(|| format!("Item #{id}")),
        icon_url: get_cached_asset_data_uri(&raw_icon).await,
        price_total: info.map(|i| i.price_total).unwrap_or(0),
        item_class: info.and_then(|i| i.item_class.clone()),
        description: info.and_then(|i| i.description.clone()),
        win_rate,
        wpa: win_rate - champion_win_rate,
        games,
        pick_rate: calc_item_pick_rate(games, total_games, is_boot),
        situational_tag: get_situational_tag(id, info).map(|s| s.to_string()),
    }
}

async fn mayhem_spell(
    id: u64,
    catalog: &HashMap<u64, CachedSpellInfo>,
    individual: &HashMap<u64, (i64, i64)>,
    champion_win_rate: f64,
) -> MayhemBuildSpell {
    let info = catalog.get(&id);
    let raw_icon = info.map(|s| s.icon_url.clone()).unwrap_or_default();
    let (games, wins) = individual.get(&id).copied().unwrap_or((0, 0));
    let win_rate = if games > 0 {
        wins as f64 / games as f64
    } else {
        champion_win_rate
    };
    MayhemBuildSpell {
        id,
        name: info
            .map(|s| s.name.clone())
            .unwrap_or_else(|| format!("Spell #{id}")),
        icon_url: get_cached_asset_data_uri(&raw_icon).await,
        description: info.and_then(|s| s.description.clone()),
        win_rate,
        wpa: win_rate - champion_win_rate,
        games: games as usize,
    }
}

async fn mayhem_augment(
    id: &str,
    catalog: &HashMap<String, AugmentCatalogEntry>,
    agg: Option<&MetricAggregate>,
    champion_win_rate: f64,
) -> MayhemBuildAugment {
    let info = catalog.get(id).or_else(|| catalog.get(&id.to_lowercase()));
    let raw_icon = info.map(|a| a.icon_url.clone()).unwrap_or_default();
    let (games, wins) = agg
        .map(|a| (a.games as usize, a.wins as f64))
        .unwrap_or((0, 0.0));
    let win_rate = if games > 0 {
        wins / games as f64
    } else {
        champion_win_rate
    };
    MayhemBuildAugment {
        id: id.to_string(),
        name: info
            .map(|a| a.name.clone())
            .unwrap_or_else(|| format!("Augment #{id}")),
        icon_url: get_cached_asset_data_uri(&raw_icon).await,
        rarity: info.and_then(|a| a.rarity.clone()),
        tooltip: info.and_then(|a| a.tooltip.clone()),
        win_rate,
        wpa: win_rate - champion_win_rate,
        games,
        pick_rate: 0.0,
    }
}

// ponytail: V2 build creation algorithm inferring purchase priority from early-ended matches and branching adaptive augment synergies.

#[derive(Clone, Debug)]
struct V2ParsedFact {
    legendaries: Vec<(i64, usize)>, // (item_id, slot_idx)
    boot: Option<i64>,
    starter: Option<i64>,
    win: bool,
    spell1: u64,
    spell2: u64,
    augments: Vec<String>,
}

fn parse_facts_for_v2(
    facts: &[ParticipantFact],
    items_catalog: &HashMap<i32, CachedItemInfo>,
) -> Vec<V2ParsedFact> {
    facts
        .iter()
        .map(|f| {
            let mut legendaries = Vec::new();
            let mut boot = None;
            let mut starter = None;

            for (slot, &it) in f.item_ids.iter().enumerate() {
                if it == 0 || it == 2050 || it == 2052 || it == 220013 {
                    continue;
                }
                let info = items_catalog.get(&(it as i32));
                if is_excluded_mayhem_item(it, info) {
                    continue;
                }
                if is_starter_item(it, info) {
                    if starter.is_none() {
                        starter = Some(it);
                    }
                    continue;
                }
                let is_boot = info
                    .map(|i| {
                        i.item_class.as_deref() == Some("boot")
                            || i.categories.iter().any(|c| c.eq_ignore_ascii_case("boots"))
                    })
                    .unwrap_or(false);
                if is_boot {
                    if boot.is_none() {
                        boot = Some(it);
                    }
                    continue;
                }
                if info
                    .map(|i| i.price_total >= 1800 || i.item_class.as_deref() == Some("legendary"))
                    .unwrap_or(true)
                {
                    legendaries.push((it, slot));
                }
            }

            V2ParsedFact {
                legendaries,
                boot,
                starter,
                win: f.win,
                spell1: f.spell1_id,
                spell2: f.spell2_id,
                augments: f.augment_ids.clone(),
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
struct RushItemCandidate {
    item_id: i64,
    rush_score: f64,
    bayesian_wr: f64,
    total_games: usize,
    solo_count: usize,
    mean_slot: f64,
}

fn infer_v2_rush_items_from_refs(
    parsed: &[&V2ParsedFact],
    champion_win_rate: f64,
    min_games: usize,
) -> Vec<RushItemCandidate> {
    let mut solo_counts: HashMap<i64, usize> = HashMap::new();
    let mut pair_first_counts: HashMap<i64, usize> = HashMap::new();
    let mut slot_sums: HashMap<i64, (usize, usize)> = HashMap::new();
    let mut item_wins: HashMap<i64, usize> = HashMap::new();
    let mut item_games: HashMap<i64, usize> = HashMap::new();

    for p in parsed {
        // Matches where player finished only 1 legendary item: 100% certainty 1st purchase!
        if p.legendaries.len() == 1 {
            let it = p.legendaries[0].0;
            *solo_counts.entry(it).or_default() += 1;
        } else if p.legendaries.len() == 2 {
            let (it1, s1) = p.legendaries[0];
            let (it2, s2) = p.legendaries[1];
            let first_it = if s1 <= s2 { it1 } else { it2 };
            *pair_first_counts.entry(first_it).or_default() += 1;
        }

        for &(it, slot) in &p.legendaries {
            let entry = slot_sums.entry(it).or_default();
            entry.0 += slot;
            entry.1 += 1;
            *item_games.entry(it).or_default() += 1;
            if p.win {
                *item_wins.entry(it).or_default() += 1;
            }
        }
    }

    let mut candidates = Vec::new();
    for (&it, &games) in &item_games {
        if games < min_games {
            continue;
        }
        let wins = item_wins.get(&it).copied().unwrap_or(0);
        let solo = solo_counts.get(&it).copied().unwrap_or(0);
        let pair1 = pair_first_counts.get(&it).copied().unwrap_or(0);
        let (s_sum, s_count) = slot_sums.get(&it).copied().unwrap_or((0, 1));
        let mean_slot = s_sum as f64 / s_count.max(1) as f64;

        let bayes_wr = (wins as f64 + 6.0 * champion_win_rate) / (games as f64 + 6.0);
        // Early signals: solo completions (weight 4.0), 2-item 1st slots (weight 1.5)
        let early_signal = (solo as f64 * 4.0) + (pair1 as f64 * 1.5);
        let slot_factor = (5.5 - mean_slot).clamp(0.2, 5.0);
        let rush_score = early_signal * slot_factor
            + (bayes_wr - champion_win_rate) * 20.0
            + (games as f64).sqrt() * 0.5;

        candidates.push(RushItemCandidate {
            item_id: it,
            rush_score,
            bayesian_wr: bayes_wr,
            total_games: games,
            solo_count: solo,
            mean_slot,
        });
    }

    candidates.sort_by(|a, b| {
        b.rush_score
            .partial_cmp(&a.rush_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates
}

async fn enrich_v2_item_from_refs(
    id: i64,
    facts: &[&V2ParsedFact],
    items_catalog: &HashMap<i32, CachedItemInfo>,
    champion_win_rate: f64,
    total_champ_games: usize,
    item_aug_synergies: &HashMap<i64, f64>,
) -> MayhemBuildItem {
    let mut it_games = 0usize;
    let mut it_wins = 0usize;
    for p in facts {
        if p.legendaries.iter().any(|(x, _)| *x == id) {
            it_games += 1;
            if p.win {
                it_wins += 1;
            }
        }
    }
    let info = items_catalog.get(&(id as i32));
    let is_boot = info
        .map(|i| {
            i.item_class.as_deref() == Some("boot")
                || i.categories.iter().any(|c| c.eq_ignore_ascii_case("boots"))
        })
        .unwrap_or(false);
    let mut it = mayhem_item(
        id,
        items_catalog,
        None,
        champion_win_rate,
        total_champ_games,
    )
    .await;
    it.games = it_games;
    it.win_rate = if it_games > 0 {
        it_wins as f64 / it_games as f64
    } else {
        champion_win_rate
    };
    it.wpa = it.win_rate - champion_win_rate;
    it.pick_rate = calc_item_pick_rate(it_games, total_champ_games, is_boot);
    if let Some(&syn) = item_aug_synergies.get(&id) {
        if syn >= 0.015 {
            it.situational_tag = Some(format!("Synergy: {:+.0}% WR", syn * 100.0));
        }
    }
    if it.situational_tag.is_none() {
        it.situational_tag = get_situational_tag(id, info).map(|s| s.to_string());
    }
    it
}

async fn assemble_v2_strategy_for_core(
    strategy_id: &str,
    label_prefix: &str,
    rush_item: i64,
    facts: &[&V2ParsedFact],
    champion_win_rate: f64,
    total_champ_games: usize,
    items_catalog: &HashMap<i32, CachedItemInfo>,
    aug_catalog: &HashMap<String, AugmentCatalogEntry>,
    item_aug_synergies: &HashMap<i64, f64>,
    shared_spells: &[MayhemBuildSpell],
    all_spells: &[MayhemBuildSpell],
    normalized_augments: &[String],
) -> MayhemBuildStrategy {
    let mut ordered_core = vec![rush_item];
    let mut exclusive_groups: HashSet<&'static str> = HashSet::new();
    if let Some(grp) = get_item_exclusive_group(rush_item, items_catalog.get(&(rush_item as i32))) {
        exclusive_groups.insert(grp);
    }

    let facts_with_rush: Vec<&V2ParsedFact> = facts
        .iter()
        .copied()
        .filter(|p| p.legendaries.iter().any(|(x, _)| *x == rush_item))
        .collect();

    // 1. Find 2nd Core Item
    let mut pair_counts: HashMap<i64, (usize, usize)> = HashMap::new();
    for p in &facts_with_rush {
        for &(it, _) in &p.legendaries {
            if it != rush_item {
                let grp = get_item_exclusive_group(it, items_catalog.get(&(it as i32)));
                if grp.map_or(false, |g| exclusive_groups.contains(g)) {
                    continue;
                }
                let e = pair_counts.entry(it).or_default();
                e.0 += 1;
                if p.win {
                    e.1 += 1;
                }
            }
        }
    }
    let mut scored_second: Vec<(i64, f64)> = pair_counts
        .into_iter()
        .map(|(it, (g, w))| {
            let bayes_wr = (w as f64 + 4.0 * champion_win_rate) / (g as f64 + 4.0);
            let co_occur = g as f64 / facts_with_rush.len().max(1) as f64;
            let syn = item_aug_synergies.get(&it).copied().unwrap_or(0.0);
            let score = (bayes_wr - champion_win_rate) * 0.5 + co_occur * 0.35 + syn * 0.20;
            (it, score)
        })
        .collect();
    scored_second.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if let Some(&(second_it, _)) = scored_second.first() {
        ordered_core.push(second_it);
        if let Some(grp) =
            get_item_exclusive_group(second_it, items_catalog.get(&(second_it as i32)))
        {
            exclusive_groups.insert(grp);
        }
    }

    // 2. Find 3rd Core Item
    let second_item_opt = ordered_core.get(1).copied();
    let facts_with_core2: Vec<&V2ParsedFact> = if let Some(s2) = second_item_opt {
        facts_with_rush
            .iter()
            .copied()
            .filter(|p| p.legendaries.iter().any(|(x, _)| *x == s2))
            .collect()
    } else {
        facts_with_rush.clone()
    };

    let sample_pool_for_3rd = if facts_with_core2.len() >= 3 {
        &facts_with_core2
    } else {
        &facts_with_rush
    };

    let mut triple_counts: HashMap<i64, (usize, usize)> = HashMap::new();
    for p in sample_pool_for_3rd {
        for &(it, _) in &p.legendaries {
            if !ordered_core.contains(&it) {
                let grp = get_item_exclusive_group(it, items_catalog.get(&(it as i32)));
                if grp.map_or(false, |g| exclusive_groups.contains(g)) {
                    continue;
                }
                let e = triple_counts.entry(it).or_default();
                e.0 += 1;
                if p.win {
                    e.1 += 1;
                }
            }
        }
    }
    let mut scored_third: Vec<(i64, f64)> = triple_counts
        .into_iter()
        .map(|(it, (g, w))| {
            let bayes_wr = (w as f64 + 4.0 * champion_win_rate) / (g as f64 + 4.0);
            let co_occur = g as f64 / sample_pool_for_3rd.len().max(1) as f64;
            let syn = item_aug_synergies.get(&it).copied().unwrap_or(0.0);
            let score = (bayes_wr - champion_win_rate) * 0.5 + co_occur * 0.35 + syn * 0.20;
            (it, score)
        })
        .collect();
    scored_third.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if let Some(&(third_it, _)) = scored_third.first() {
        ordered_core.push(third_it);
        if let Some(grp) = get_item_exclusive_group(third_it, items_catalog.get(&(third_it as i32)))
        {
            exclusive_groups.insert(grp);
        }
    }

    // 3. Find Boot and Starter
    let mut boot_counts: HashMap<i64, (usize, usize)> = HashMap::new();
    let mut starter_counts: HashMap<i64, (usize, usize)> = HashMap::new();
    for p in &facts_with_rush {
        if let Some(b) = p.boot {
            let e = boot_counts.entry(b).or_default();
            e.0 += 1;
            if p.win {
                e.1 += 1;
            }
        }
        if let Some(s) = p.starter {
            let e = starter_counts.entry(s).or_default();
            e.0 += 1;
            if p.win {
                e.1 += 1;
            }
        }
    }
    let boot_id = boot_counts.into_iter().max_by_key(|x| x.1 .0).map(|x| x.0);
    let starter_id = starter_counts
        .into_iter()
        .max_by_key(|x| x.1 .0)
        .and_then(|(id, (g, _))| {
            if g >= 2 && (g as f64 / facts_with_rush.len().max(1) as f64) >= 0.05 {
                Some(id)
            } else {
                None
            }
        });

    // 4. Find Continuation / Capstones (Items 4 & 5)
    let mut continuation_counts: HashMap<i64, (usize, usize)> = HashMap::new();
    for p in &facts_with_rush {
        for &(it, _) in &p.legendaries {
            if !ordered_core.contains(&it) {
                let grp = get_item_exclusive_group(it, items_catalog.get(&(it as i32)));
                if grp.map_or(false, |g| exclusive_groups.contains(g)) {
                    continue;
                }
                let e = continuation_counts.entry(it).or_default();
                e.0 += 1;
                if p.win {
                    e.1 += 1;
                }
            }
        }
    }
    let mut scored_continuation: Vec<(i64, f64, usize, usize)> = continuation_counts
        .into_iter()
        .map(|(it, (g, w))| {
            let bayes_wr = (w as f64 + 4.0 * champion_win_rate) / (g as f64 + 4.0);
            let co_occur = g as f64 / facts_with_rush.len().max(1) as f64;
            let syn = item_aug_synergies.get(&it).copied().unwrap_or(0.0);
            let score = (bayes_wr - champion_win_rate) * 0.4 + co_occur * 0.4 + syn * 0.25;
            (it, score, g, w)
        })
        .collect();
    scored_continuation.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut continuation_ids = Vec::new();
    for (it, _, _, _) in &scored_continuation {
        let grp = get_item_exclusive_group(*it, items_catalog.get(&(*it as i32)));
        if grp.map_or(false, |g| exclusive_groups.contains(g)) {
            continue;
        }
        if let Some(g) = grp {
            exclusive_groups.insert(g);
        }
        continuation_ids.push(*it);
        if continuation_ids.len() >= 2 {
            break;
        }
    }

    // 5. Construct 6 ordered build items
    let mut build_item_ids = Vec::new();
    // Item 1: Inferred 1st Rush Item
    build_item_ids.push(ordered_core[0]);
    // Item 2: Boots
    if let Some(b) = boot_id {
        build_item_ids.push(b);
    }
    // Item 3: 2nd Core Item
    if ordered_core.len() > 1 {
        build_item_ids.push(ordered_core[1]);
    }
    // Item 4: 3rd Core Item
    if ordered_core.len() > 2 {
        build_item_ids.push(ordered_core[2]);
    }
    // Items 5 & 6: Continuation / Capstones
    for &c_id in &continuation_ids {
        if build_item_ids.len() >= 6 {
            break;
        }
        if !build_item_ids.contains(&c_id) {
            build_item_ids.push(c_id);
        }
    }

    let mut items = Vec::new();
    for &id in &build_item_ids {
        items.push(
            enrich_v2_item_from_refs(
                id,
                facts,
                items_catalog,
                champion_win_rate,
                total_champ_games,
                item_aug_synergies,
            )
            .await,
        );
    }

    // Starter item
    let starter_item = if let Some(sid) = starter_id {
        Some(
            enrich_v2_item_from_refs(
                sid,
                facts,
                items_catalog,
                champion_win_rate,
                total_champ_games,
                item_aug_synergies,
            )
            .await,
        )
    } else {
        None
    };

    // Alternative & Situational items
    let mut alternative_items = Vec::new();
    let mut situational_items = Vec::new();

    for (c_id, _, _, _) in scored_continuation.into_iter().skip(continuation_ids.len()) {
        if build_item_ids.contains(&c_id) {
            continue;
        }
        let item = enrich_v2_item_from_refs(
            c_id,
            facts,
            items_catalog,
            champion_win_rate,
            total_champ_games,
            item_aug_synergies,
        )
        .await;
        let is_situational = item
            .situational_tag
            .as_ref()
            .map_or(false, |t| !t.starts_with("Synergy:"));
        if is_situational && situational_items.len() < 12 {
            situational_items.push(item);
        } else if alternative_items.len() < 24 {
            alternative_items.push(item);
        }
    }

    // Sort alternative items so items with positive augment synergy are at the front
    alternative_items.sort_by(|a, b| {
        let syn_a = item_aug_synergies.get(&a.id).copied().unwrap_or(0.0);
        let syn_b = item_aug_synergies.get(&b.id).copied().unwrap_or(0.0);
        syn_b
            .partial_cmp(&syn_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let stages = build_stage_pathing(&starter_item, &items, &situational_items);

    // Build statistics
    let core_facts: Vec<&V2ParsedFact> = facts
        .iter()
        .copied()
        .filter(|p| {
            ordered_core
                .iter()
                .all(|c| p.legendaries.iter().any(|(x, _)| x == c))
        })
        .collect();
    let sample_games = core_facts.len().max(1);
    let sample_wins = core_facts.iter().filter(|p| p.win).count();
    let build_win_rate = if core_facts.is_empty() {
        champion_win_rate
    } else {
        (sample_wins as f64 + 5.0 * champion_win_rate) / (sample_games as f64 + 5.0)
    };
    let build_wpa = build_win_rate - champion_win_rate;

    // Mine Top Augments
    let mut aug_counts: HashMap<String, (usize, usize)> = HashMap::new();
    for p in &core_facts {
        for a in &p.augments {
            if !a.is_empty()
                && a != "0"
                && !normalized_augments
                    .iter()
                    .any(|r| r.eq_ignore_ascii_case(a))
            {
                let e = aug_counts.entry(a.clone()).or_default();
                e.0 += 1;
                if p.win {
                    e.1 += 1;
                }
            }
        }
    }
    let mut aug_list: Vec<(String, usize, usize)> = aug_counts
        .into_iter()
        .map(|(a, (g, w))| (a, g, w))
        .collect();
    aug_list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2)));

    let mut augments = Vec::new();
    for (a_id, g, w) in aug_list.into_iter().take(6) {
        let mut aug = mayhem_augment(&a_id, aug_catalog, None, champion_win_rate).await;
        aug.games = g;
        aug.win_rate = (w as f64 + 3.0 * build_win_rate) / (g as f64 + 3.0);
        aug.wpa = aug.win_rate - champion_win_rate;
        aug.pick_rate = g as f64 / sample_games as f64;
        augments.push(aug);
    }

    let prefix = classify_archetype_prefix(&ordered_core, items_catalog);
    let names = generate_archetype_label(&ordered_core, items_catalog);
    let label = format!("{}: {}: {}", label_prefix, prefix, names);

    MayhemBuildStrategy {
        id: strategy_id.to_string(),
        label,
        metric_name: "Win Rate".to_string(),
        avg_metric_value: format!("{:+.1}% WPA", build_wpa * 100.0),
        raw_metric_value: build_win_rate,
        build_win_rate,
        build_wpa,
        starter_item,
        items,
        alternative_items,
        situational_items,
        stages,
        spells: shared_spells.to_vec(),
        all_spells: all_spells.to_vec(),
        augments,
        sample_games,
    }
}

async fn assemble_v2_strategy_from_mined(
    arch: &MinedArchetype,
    facts: &[V2ParsedFact],
    champion_win_rate: f64,
    total_champ_games: usize,
    items_catalog: &HashMap<i32, CachedItemInfo>,
    aug_catalog: &HashMap<String, AugmentCatalogEntry>,
    item_aug_synergies: &HashMap<i64, f64>,
    shared_spells: &[MayhemBuildSpell],
    all_spells: &[MayhemBuildSpell],
    normalized_augments: &[String],
) -> MayhemBuildStrategy {
    let fact_refs: Vec<&V2ParsedFact> = facts.iter().collect();

    let mut items = Vec::new();
    for &id in &arch.core_items {
        items.push(
            enrich_v2_item_from_refs(
                id,
                &fact_refs,
                items_catalog,
                champion_win_rate,
                total_champ_games,
                item_aug_synergies,
            )
            .await,
        );
    }
    if let Some(bid) = arch.boot_id {
        if items.len() >= 1 {
            items.insert(
                1,
                enrich_v2_item_from_refs(
                    bid,
                    &fact_refs,
                    items_catalog,
                    champion_win_rate,
                    total_champ_games,
                    item_aug_synergies,
                )
                .await,
            );
        } else {
            items.push(
                enrich_v2_item_from_refs(
                    bid,
                    &fact_refs,
                    items_catalog,
                    champion_win_rate,
                    total_champ_games,
                    item_aug_synergies,
                )
                .await,
            );
        }
    }
    for &(id, _, _, _) in &arch.continuation_items {
        if items.len() >= 6 {
            break;
        }
        if !items.iter().any(|x| x.id == id) {
            items.push(
                enrich_v2_item_from_refs(
                    id,
                    &fact_refs,
                    items_catalog,
                    champion_win_rate,
                    total_champ_games,
                    item_aug_synergies,
                )
                .await,
            );
        }
    }

    let starter_item = if let Some(sid) = arch.starter_id {
        Some(
            enrich_v2_item_from_refs(
                sid,
                &fact_refs,
                items_catalog,
                champion_win_rate,
                total_champ_games,
                item_aug_synergies,
            )
            .await,
        )
    } else {
        None
    };

    let mut alternative_items = Vec::new();
    for &(id, _, _, _) in arch.continuation_items.iter().skip(2) {
        if !items.iter().any(|x| x.id == id) && alternative_items.len() < 20 {
            alternative_items.push(
                enrich_v2_item_from_refs(
                    id,
                    &fact_refs,
                    items_catalog,
                    champion_win_rate,
                    total_champ_games,
                    item_aug_synergies,
                )
                .await,
            );
        }
    }
    alternative_items.sort_by(|a, b| {
        let syn_a = item_aug_synergies.get(&a.id).copied().unwrap_or(0.0);
        let syn_b = item_aug_synergies.get(&b.id).copied().unwrap_or(0.0);
        syn_b
            .partial_cmp(&syn_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut situational_items = Vec::new();
    for &(id, _, _, _) in &arch.situational_items {
        if !items.iter().any(|x| x.id == id) && situational_items.len() < 12 {
            situational_items.push(
                enrich_v2_item_from_refs(
                    id,
                    &fact_refs,
                    items_catalog,
                    champion_win_rate,
                    total_champ_games,
                    item_aug_synergies,
                )
                .await,
            );
        }
    }

    let stages = build_stage_pathing(&starter_item, &items, &situational_items);

    let mut augments = Vec::new();
    for (id, g, wr, wpa, pr) in &arch.top_augments {
        if normalized_augments
            .iter()
            .any(|r| r.eq_ignore_ascii_case(id))
        {
            continue;
        }
        let mut a = mayhem_augment(id, aug_catalog, None, champion_win_rate).await;
        a.games = *g;
        a.win_rate = *wr;
        a.wpa = *wpa;
        a.pick_rate = *pr;
        augments.push(a);
    }

    MayhemBuildStrategy {
        id: arch.id.clone(),
        label: format!("Adaptive: {}", arch.label),
        metric_name: "Win Rate".to_string(),
        avg_metric_value: format!("{:+.1}% WPA", arch.build_wpa * 100.0),
        raw_metric_value: arch.build_win_rate,
        build_win_rate: arch.build_win_rate,
        build_wpa: arch.build_wpa,
        starter_item,
        items,
        alternative_items,
        situational_items,
        stages,
        spells: shared_spells.to_vec(),
        all_spells: all_spells.to_vec(),
        augments,
        sample_games: arch.sample_games,
    }
}

async fn compute_mayhem_champion_build_v2(
    champion_id: i64,
    champion_name: &str,
    icon_url: &str,
    facts: &[ParticipantFact],
    _global_baseline: f64,
    champion_win_rate: f64,
    champion_wpa: f64,
    normalized_augments: &[String],
    items_catalog: &HashMap<i32, CachedItemInfo>,
    aug_catalog: &HashMap<String, AugmentCatalogEntry>,
    spells_catalog: &HashMap<u64, CachedSpellInfo>,
) -> MayhemChampionBuildSummary {
    let total_games = facts.len();
    let parsed = parse_facts_for_v2(facts, items_catalog);

    // 1. Cache champion augments map for fast server-side pagination & filtering
    {
        let mut aug_map = HashMap::new();
        for p in &parsed {
            for a in &p.augments {
                if !a.is_empty() && a != "0" {
                    let entry = aug_map.entry(a.clone()).or_insert((0usize, 0usize));
                    entry.0 += 1;
                    if p.win {
                        entry.1 += 1;
                    }
                }
            }
        }
        let mut aug_cache = MAYHEM_CHAMPION_AUGMENTS_CACHE.write().await;
        aug_cache.insert(
            (champion_id, normalized_augments.to_vec()),
            (Instant::now(), champion_win_rate, facts.len(), aug_map),
        );
    }

    // 2. Compute empirical augment synergies from facts
    let mut item_aug_synergies: HashMap<i64, f64> = HashMap::new();
    for aug in normalized_augments {
        let aug_facts: Vec<&V2ParsedFact> = parsed
            .iter()
            .filter(|p| p.augments.iter().any(|a| a.eq_ignore_ascii_case(aug)))
            .collect();
        if aug_facts.len() >= 2 {
            let mut counts: HashMap<i64, (usize, usize)> = HashMap::new();
            for p in &aug_facts {
                for &(it, _) in &p.legendaries {
                    let e = counts.entry(it).or_default();
                    e.0 += 1;
                    if p.win {
                        e.1 += 1;
                    }
                }
            }
            for (it, (g, w)) in counts {
                if g >= 2 {
                    let wr = (w as f64 + 3.0 * champion_win_rate) / (g as f64 + 3.0);
                    let delta = wr - champion_win_rate;
                    if delta > 0.01 {
                        *item_aug_synergies.entry(it).or_default() += delta;
                    }
                }
            }
        }
    }

    // 3. Shared spell statistics across all facts
    let mut spell_pairs: HashMap<(u64, u64), (usize, usize)> = HashMap::new();
    let mut ind_spells: HashMap<u64, (i64, i64)> = HashMap::new();
    for p in &parsed {
        if p.spell1 > 0 && p.spell2 > 0 {
            let pair = (p.spell1.min(p.spell2), p.spell1.max(p.spell2));
            let e = spell_pairs.entry(pair).or_default();
            e.0 += 1;
            if p.win {
                e.1 += 1;
            }
        }
        for sp in [p.spell1, p.spell2] {
            if sp > 0 {
                let e = ind_spells.entry(sp).or_default();
                e.0 += 1;
                if p.win {
                    e.1 += 1;
                }
            }
        }
    }
    let top_spell_pair = spell_pairs
        .into_iter()
        .max_by_key(|x| x.1 .0)
        .map(|x| x.0)
        .unwrap_or((4, 32));

    let mut shared_spells = Vec::new();
    for sp in [top_spell_pair.0, top_spell_pair.1] {
        shared_spells.push(mayhem_spell(sp, spells_catalog, &ind_spells, champion_win_rate).await);
    }
    let all_spells = shared_spells.clone();

    // 4. Infer Rush Items by analyzing incomplete games (solo completions & slot positions)
    let min_rush_games = if total_games <= 15 {
        1
    } else if total_games <= 50 {
        2
    } else {
        3
    };
    let parsed_refs: Vec<&V2ParsedFact> = parsed.iter().collect();
    let candidates = infer_v2_rush_items_from_refs(&parsed_refs, champion_win_rate, min_rush_games);

    let mut strategies = Vec::new();

    // 5. Build Adaptive Strategies (when augments are selected)
    let mut adaptive_strategies = Vec::new();
    if !normalized_augments.is_empty() {
        for aug in normalized_augments {
            // A. Check for known warped archetype template
            if let Some(warped_arch) = build_augment_warped_archetype(aug, champion_win_rate) {
                let strat = assemble_v2_strategy_from_mined(
                    &warped_arch,
                    &parsed,
                    champion_win_rate,
                    total_games,
                    items_catalog,
                    aug_catalog,
                    &item_aug_synergies,
                    &shared_spells,
                    &all_spells,
                    normalized_augments,
                )
                .await;
                adaptive_strategies.push(strat);
            }

            // B. Data-driven augment core
            let aug_facts: Vec<&V2ParsedFact> = parsed
                .iter()
                .filter(|p| p.augments.iter().any(|a| a.eq_ignore_ascii_case(aug)))
                .collect();
            if aug_facts.len() >= 3 {
                let aug_candidates =
                    infer_v2_rush_items_from_refs(&aug_facts, champion_win_rate, 2);
                if let Some(top_aug_cand) = aug_candidates.first() {
                    let strat = assemble_v2_strategy_for_core(
                        &format!("adaptive_{}", aug.to_lowercase().replace(' ', "_")),
                        &format!("Adaptive: {}", aug),
                        top_aug_cand.item_id,
                        &aug_facts,
                        champion_win_rate,
                        total_games,
                        items_catalog,
                        aug_catalog,
                        &item_aug_synergies,
                        &shared_spells,
                        &all_spells,
                        normalized_augments,
                    )
                    .await;
                    if !adaptive_strategies.iter().any(|s| {
                        s.items.iter().map(|i| i.id).collect::<Vec<_>>()
                            == strat.items.iter().map(|i| i.id).collect::<Vec<_>>()
                    }) {
                        adaptive_strategies.push(strat);
                    }
                }
            }
        }
    }

    // 6. Build General Builds (1 or 2 robust paths for all games)
    if let Some(first_cand) = candidates.first() {
        let general_1 = assemble_v2_strategy_for_core(
            "general_1",
            "General",
            first_cand.item_id,
            &parsed_refs,
            champion_win_rate,
            total_games,
            items_catalog,
            aug_catalog,
            &item_aug_synergies,
            &shared_spells,
            &all_spells,
            normalized_augments,
        )
        .await;

        let g1_core_items: HashSet<i64> = general_1.items.iter().take(3).map(|i| i.id).collect();

        // Check for viable distinct General Build 2 (distinct rush item / different archetype)
        let mut general_2 = None;
        for cand in candidates.iter().skip(1) {
            if cand.total_games >= 4 && !g1_core_items.contains(&cand.item_id) {
                let strat = assemble_v2_strategy_for_core(
                    "general_2",
                    "General",
                    cand.item_id,
                    &parsed_refs,
                    champion_win_rate,
                    total_games,
                    items_catalog,
                    aug_catalog,
                    &item_aug_synergies,
                    &shared_spells,
                    &all_spells,
                    normalized_augments,
                )
                .await;
                let g2_core: HashSet<i64> = strat.items.iter().take(3).map(|i| i.id).collect();
                if g2_core.intersection(&g1_core_items).count() <= 1 {
                    general_2 = Some(strat);
                    break;
                }
            }
        }

        if !adaptive_strategies.is_empty() {
            strategies.extend(adaptive_strategies);
            strategies.push(general_1);
            if let Some(g2) = general_2 {
                strategies.push(g2);
            }
        } else {
            strategies.push(general_1);
            if let Some(g2) = general_2 {
                strategies.push(g2);
            }
        }
    }

    // Fallback if strategies empty:
    if strategies.is_empty() {
        for (idx, arch) in mine_build_archetypes(
            &facts.iter().collect::<Vec<_>>(),
            items_catalog,
            champion_win_rate,
        )
        .into_iter()
        .enumerate()
        {
            let strat = assemble_v2_strategy_from_mined(
                &arch,
                &parsed,
                champion_win_rate,
                total_games,
                items_catalog,
                aug_catalog,
                &item_aug_synergies,
                &shared_spells,
                &all_spells,
                normalized_augments,
            )
            .await;
            strategies.push(strat);
            if idx >= 1 {
                break;
            }
        }
    }

    MayhemChampionBuildSummary {
        champion_id,
        champion_name: champion_name.to_string(),
        icon_url: icon_url.to_string(),
        champion_win_rate,
        champion_wpa,
        total_games,
        strategies,
        applied_augment_ids: normalized_augments.to_vec(),
        fallback_used: false,
    }
}

pub async fn compute_mayhem_champion_build(
    champion_id: i64,
    requested_augments: Vec<String>,
    algorithm_version: Option<String>,
) -> Result<MayhemChampionBuildSummary, String> {
    let version = algorithm_version
        .unwrap_or_else(|| "v2".to_string())
        .to_lowercase();
    let version = if version == "v1" {
        "v1".to_string()
    } else {
        "v2".to_string()
    };

    let mut normalized_augments: Vec<String> = requested_augments
        .into_iter()
        .filter(|a| !a.is_empty() && a != "0")
        .collect();
    normalized_augments.sort();
    normalized_augments.dedup();

    let cache_key = (champion_id, normalized_augments.clone(), version.clone());
    // 1. Check in-memory cache
    {
        let cache = MAYHEM_CHAMPION_BUILDS_CACHE.read().await;
        if let Some((created_at, summary)) = cache.get(&cache_key) {
            if created_at.elapsed() < std::time::Duration::from_secs(120) {
                return Ok(summary.clone());
            }
        }
    }

    let db_path = history_data_dir().join("global-aram.db");
    if !db_path.exists() {
        return Err("global-aram.db not found".to_string());
    }

    // 2. Query participant facts in blocking task
    let facts_res = tokio::task::spawn_blocking(move || -> Result<(Vec<ParticipantFact>, f64), String> {
        let connection = Connection::open(&db_path)
            .map_err(|e| format!("failed opening global-aram.db: {e}"))?;
        let _ = connection.busy_timeout(std::time::Duration::from_secs(5));
        let _ = connection.pragma_update(None, "journal_mode", "WAL");

        let global_baseline: f64 = connection
            .query_row("SELECT AVG(win) FROM aram_participants", [], |row| row.get(0))
            .unwrap_or(0.50);

        let mut stmt = connection
            .prepare(
                "SELECT win, item_ids_json, augment_ids_json, spell1_id, spell2_id,
                        damage, damage_taken, true_damage, damage_mitigated, total_heal, turret_damage,
                        game_duration
                 FROM aram_participants
                 WHERE champion_id = ?1",
            )
            .map_err(|e| format!("failed preparing participants query: {e}"))?;

        let rows = stmt
            .query_map([champion_id], |row| {
                let it_json: String = row.get(1)?;
                let aug_json: String = row.get(2)?;
                let item_ids: Vec<i64> = serde_json::from_str(&it_json).unwrap_or_default();
                let augment_ids: Vec<String> = serde_json::from_str(&aug_json).unwrap_or_default();
                Ok(ParticipantFact {
                    win: row.get::<_, i64>(0)? != 0,
                    item_ids,
                    augment_ids,
                    spell1_id: row.get::<_, i64>(3).unwrap_or(0) as u64,
                    spell2_id: row.get::<_, i64>(4).unwrap_or(0) as u64,
                    damage: row.get(5).unwrap_or(0),
                    damage_taken: row.get(6).unwrap_or(0),
                    true_damage: row.get(7).unwrap_or(0),
                    damage_mitigated: row.get(8).unwrap_or(0),
                    total_heal: row.get(9).unwrap_or(0),
                    turret_damage: row.get(10).unwrap_or(0),
                    game_duration: row.get(11).unwrap_or(1200),
                })
            })
            .map_err(|e| format!("failed querying facts: {e}"))?;

        let facts: Vec<ParticipantFact> = rows.filter_map(Result::ok).collect();
        Ok((facts, global_baseline))
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))??;

    let (facts, global_baseline) = facts_res;
    let total_games = facts.len();

    // 3. Resolve champion identity
    let all_champs = get_all_champions().await.unwrap_or_default();
    let champ_info = all_champs.iter().find(|c| c.id == champion_id);
    let champion_name = champ_info
        .map(|c| c.name.clone())
        .unwrap_or_else(|| format!("Champion #{champion_id}"));
    let champ_icon = champ_info
        .map(|c| c.square_icon_url.clone())
        .unwrap_or_default();
    let icon_url = get_cached_asset_data_uri(&champ_icon).await;

    if total_games == 0 {
        return Ok(MayhemChampionBuildSummary {
            champion_id,
            champion_name,
            icon_url,
            champion_win_rate: 0.50,
            champion_wpa: 0.0,
            total_games: 0,
            strategies: Vec::new(),
            applied_augment_ids: Vec::new(),
            fallback_used: false,
        });
    }

    let wins_count = facts.iter().filter(|f| f.win).count();
    let champion_win_rate = wins_count as f64 / total_games as f64;
    let champion_wpa = champion_win_rate - global_baseline;

    if version == "v2" {
        let items_catalog = get_items_by_id().await.unwrap_or_default();
        let aug_catalog = get_augment_catalog_map().await.unwrap_or_default();
        let spells_catalog = get_spells_by_id().await.unwrap_or_default();

        let summary = compute_mayhem_champion_build_v2(
            champion_id,
            &champion_name,
            &icon_url,
            &facts,
            global_baseline,
            champion_win_rate,
            champion_wpa,
            &normalized_augments,
            &items_catalog,
            &aug_catalog,
            &spells_catalog,
        )
        .await;

        let mut cache = MAYHEM_CHAMPION_BUILDS_CACHE.write().await;
        cache.insert(cache_key, (Instant::now(), summary.clone()));
        return Ok(summary);
    }

    // 4. Augment conditioning filter (has_all) with >= 15 games fallback
    let mut applied_augment_ids = normalized_augments.clone();
    let mut fallback_used = false;

    let mut filtered_facts: Vec<&ParticipantFact> = facts
        .iter()
        .filter(|f| {
            applied_augment_ids
                .iter()
                .all(|a| f.augment_ids.contains(a))
        })
        .collect();

    while filtered_facts.len() < 15 && !applied_augment_ids.is_empty() {
        fallback_used = true;
        applied_augment_ids.pop();
        filtered_facts = facts
            .iter()
            .filter(|f| {
                applied_augment_ids
                    .iter()
                    .all(|a| f.augment_ids.contains(a))
            })
            .collect();
    }

    let active_facts: Vec<&ParticipantFact> = if filtered_facts.is_empty() {
        facts.iter().collect()
    } else {
        filtered_facts
    };

    // Cache champion augments map for fast server-side pagination & filtering
    {
        let mut aug_map = HashMap::new();
        for f in &active_facts {
            for a in &f.augment_ids {
                if !a.is_empty() && a != "0" {
                    let entry = aug_map.entry(a.clone()).or_insert((0usize, 0usize));
                    entry.0 += 1;
                    if f.win {
                        entry.1 += 1;
                    }
                }
            }
        }
        let mut aug_cache = MAYHEM_CHAMPION_AUGMENTS_CACHE.write().await;
        aug_cache.insert(
            (champion_id, normalized_augments.clone()),
            (
                Instant::now(),
                champion_win_rate,
                active_facts.len(),
                aug_map,
            ),
        );
    }

    // Everything below aggregates the augment-conditioned sample.
    let total_games_all = total_games;
    let total_games = active_facts.len();

    // 4. Aggregate items, augments, and spell pairs
    let mut items_agg: HashMap<i64, MetricAggregate> = HashMap::new();
    let mut augments_agg: HashMap<String, MetricAggregate> = HashMap::new();
    let mut spells_agg: HashMap<(u64, u64), MetricAggregate> = HashMap::new();
    let mut individual_spells_agg: HashMap<u64, (i64, i64)> = HashMap::new();

    for f in &active_facts {
        let dur_min = (f.game_duration.max(60) as f64) / 60.0;
        let dpm = f.damage as f64 / dur_min;
        let win_val = i64::from(f.win);

        for &it in &f.item_ids {
            if it == 0 || it == 2050 || it == 2052 || it == 220013 {
                continue;
            }
            let entry = items_agg.entry(it).or_default();
            entry.games += 1;
            entry.wins += win_val;
            entry.damage += f.damage;
            entry.damage_taken += f.damage_taken;
            entry.true_damage += f.true_damage;
            entry.damage_mitigated += f.damage_mitigated;
            entry.total_heal += f.total_heal;
            entry.turret_damage += f.turret_damage;
            entry.dpm_sum += dpm;
        }

        for aug in &f.augment_ids {
            if aug.is_empty() || aug == "0" {
                continue;
            }
            let entry = augments_agg.entry(aug.clone()).or_default();
            entry.games += 1;
            entry.wins += win_val;
            entry.damage += f.damage;
            entry.damage_taken += f.damage_taken;
            entry.true_damage += f.true_damage;
            entry.damage_mitigated += f.damage_mitigated;
            entry.total_heal += f.total_heal;
            entry.turret_damage += f.turret_damage;
            entry.dpm_sum += dpm;
        }

        if f.spell1_id > 0 && f.spell2_id > 0 {
            let pair = (f.spell1_id.min(f.spell2_id), f.spell1_id.max(f.spell2_id));
            let entry = spells_agg.entry(pair).or_default();
            entry.games += 1;
            entry.wins += win_val;
            entry.damage += f.damage;
            entry.damage_taken += f.damage_taken;
            entry.true_damage += f.true_damage;
            entry.damage_mitigated += f.damage_mitigated;
            entry.total_heal += f.total_heal;
            entry.turret_damage += f.turret_damage;
            entry.dpm_sum += dpm;
        }

        for &sp in &[f.spell1_id, f.spell2_id] {
            if sp > 0 {
                let entry = individual_spells_agg.entry(sp).or_insert((0, 0));
                entry.0 += 1;
                if f.win {
                    entry.1 += 1;
                }
            }
        }
    }

    // 5. Catalogs resolution
    let items_catalog = get_items_by_id().await.unwrap_or_default();
    let aug_catalog = get_augment_catalog_map().await.unwrap_or_default();
    let spells_catalog = get_spells_by_id().await.unwrap_or_default();

    let min_item_games = if total_games <= 15 {
        1
    } else if total_games <= 50 {
        2
    } else {
        ((total_games as f64 * 0.012).ceil() as i64).clamp(3, 20)
    };
    let min_aug_games = if total_games <= 15 {
        1
    } else if total_games <= 50 {
        2
    } else {
        ((total_games as f64 * 0.008).ceil() as i64).clamp(3, 15)
    };

    let strategy_defs = [
        ("win_rate", "Highest Win Rate", "Win Rate"),
        ("damage_dealt", "Damage Dealt", "Damage Dealt"),
        ("damage_per_minute", "Damage / Min", "DPM"),
        ("true_damage", "True Damage", "True Damage"),
        ("damage_taken", "Damage Taken", "Damage Taken"),
        ("damage_mitigated", "Damage Mitigated", "Damage Mitigated"),
        ("healing", "Healing Done", "Healing Done"),
        ("turret_damage", "Turret Damage", "Turret Damage"),
    ];

    let mut strategies = Vec::new();

    for (strat_id, strat_label, metric_name) in strategy_defs {
        // A. Score and select items
        let mut starters: Vec<(i64, f64, &MetricAggregate)> = Vec::new();
        let mut boots: Vec<(i64, f64, &MetricAggregate)> = Vec::new();
        let mut core_items: Vec<(i64, f64, &MetricAggregate)> = Vec::new();

        for (&it_id, agg) in &items_agg {
            let item_info = items_catalog.get(&(it_id as i32));
            if is_excluded_mayhem_item(it_id, item_info) {
                continue;
            }

            let warp_bonus = get_item_augment_warp_bonus(it_id, &normalized_augments);
            let score = score_metric(agg, strat_id, champion_win_rate) * (1.0 + warp_bonus);
            let is_starter = is_starter_item(it_id, item_info);
            let is_boot = item_info
                .map(|i| {
                    i.item_class.as_deref() == Some("boot")
                        || i.categories.iter().any(|c| c.eq_ignore_ascii_case("boots"))
                })
                .unwrap_or(false);

            if is_starter {
                if agg.games >= 1 {
                    starters.push((it_id, score, agg));
                }
            } else if is_boot {
                if agg.games >= min_item_games
                    && item_info.map(|i| i.price_total >= 800).unwrap_or(true)
                {
                    boots.push((it_id, score, agg));
                }
            } else if item_info
                .map(|i| i.price_total >= 1400 || i.item_class.as_deref() == Some("legendary"))
                .unwrap_or(true)
            {
                if agg.games >= min_item_games {
                    core_items.push((it_id, score, agg));
                }
            }
        }

        starters.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        boots.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        core_items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 6 main items: 1st core item, then boots, then remaining non-conflicting core items up to 6
        let mut chosen_item_ids = Vec::new();
        let mut used_exclusive_groups: HashSet<&'static str> = HashSet::new();

        if let Some(&(id, _, _)) = core_items.first() {
            let item_info = items_catalog.get(&(id as i32));
            chosen_item_ids.push(id);
            if let Some(grp) = get_item_exclusive_group(id, item_info) {
                used_exclusive_groups.insert(grp);
            }
        }
        if let Some(&(id, _, _)) = boots.first() {
            let item_info = items_catalog.get(&(id as i32));
            chosen_item_ids.push(id);
            if let Some(grp) = get_item_exclusive_group(id, item_info) {
                used_exclusive_groups.insert(grp);
            }
        }
        for &(id, _, _) in core_items.iter().skip(1) {
            if chosen_item_ids.len() >= 6 {
                break;
            }
            if !chosen_item_ids.contains(&id) {
                let item_info = items_catalog.get(&(id as i32));
                if let Some(grp) = get_item_exclusive_group(id, item_info) {
                    if used_exclusive_groups.contains(grp) {
                        continue;
                    }
                    used_exclusive_groups.insert(grp);
                }
                chosen_item_ids.push(id);
            }
        }
        for &(id, _, _) in &core_items {
            if chosen_item_ids.len() >= 6 {
                break;
            }
            if !chosen_item_ids.contains(&id) {
                let item_info = items_catalog.get(&(id as i32));
                if let Some(grp) = get_item_exclusive_group(id, item_info) {
                    if used_exclusive_groups.contains(grp) {
                        continue;
                    }
                    used_exclusive_groups.insert(grp);
                }
                chosen_item_ids.push(id);
            }
        }

        // B. Starter item (shown separately above 6 items; omitted if pick rate is negligible)
        let starter_item = if let Some(&(id, _, agg)) = starters.first() {
            let pick_rate = calc_item_pick_rate(agg.games as usize, total_games, false);
            if agg.games < min_item_games || pick_rate < 0.05 {
                None
            } else {
                let item_info = items_catalog.get(&(id as i32));
                let name = item_info
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| format!("Starter #{id}"));
                let raw_icon = item_info.map(|i| i.icon_url.clone()).unwrap_or_default();
                let icon_url = get_cached_asset_data_uri(&raw_icon).await;
                let price_total = item_info.map(|i| i.price_total).unwrap_or(0);
                let item_class = item_info.and_then(|i| i.item_class.clone());
                let description = item_info.and_then(|i| i.description.clone());
                let wr = agg.wins as f64 / agg.games.max(1) as f64;
                let wpa = wr - champion_win_rate;
                Some(MayhemBuildItem {
                    id,
                    name,
                    icon_url,
                    price_total,
                    item_class,
                    description,
                    win_rate: wr,
                    wpa,
                    games: agg.games as usize,
                    pick_rate,
                    situational_tag: None,
                })
            }
        } else {
            None
        };

        // C. Alternative items (for exploration below the 6 recommended items, ordered by strategy score)
        let mut alternative_items = Vec::new();
        for &(id, _, agg) in &core_items {
            if !chosen_item_ids.contains(&id) {
                let item_info = items_catalog.get(&(id as i32));
                let name = item_info
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| format!("Item #{id}"));
                let raw_icon = item_info.map(|i| i.icon_url.clone()).unwrap_or_default();
                let icon_url = get_cached_asset_data_uri(&raw_icon).await;
                let price_total = item_info.map(|i| i.price_total).unwrap_or(0);
                let item_class = item_info.and_then(|i| i.item_class.clone());
                let description = item_info.and_then(|i| i.description.clone());
                let wr = agg.wins as f64 / agg.games.max(1) as f64;
                let wpa = wr - champion_win_rate;
                let pick_rate = calc_item_pick_rate(agg.games as usize, total_games, false);
                alternative_items.push(MayhemBuildItem {
                    id,
                    name,
                    icon_url,
                    price_total,
                    item_class,
                    description,
                    win_rate: wr,
                    wpa,
                    games: agg.games as usize,
                    pick_rate,
                    situational_tag: get_situational_tag(id, item_info).map(|s| s.to_string()),
                });
                if alternative_items.len() >= 30 {
                    break;
                }
            }
        }

        // D. Select summoner spells (Optimal Pair with Bayesian credibility and individual spell sanity check)
        let min_pair_games = if total_games <= 15 {
            1
        } else if total_games <= 50 {
            2
        } else {
            ((total_games as f64 * 0.02).ceil() as i64).clamp(5, 50)
        };

        let mut sorted_spells: Vec<((u64, u64), f64, &MetricAggregate)> = spells_agg
            .iter()
            .filter(|(pair, agg)| {
                if agg.games < min_pair_games {
                    return false;
                }
                // Check individual WPA: penalize pairs containing spells with significantly negative performance
                let sp1_wpa = individual_spells_agg
                    .get(&pair.0)
                    .map(|(g, w)| *w as f64 / (*g).max(1) as f64 - champion_win_rate)
                    .unwrap_or(0.0);
                let sp2_wpa = individual_spells_agg
                    .get(&pair.1)
                    .map(|(g, w)| *w as f64 / (*g).max(1) as f64 - champion_win_rate)
                    .unwrap_or(0.0);
                // If either spell has a strong negative WPA (< -1.5%) while viable alternatives exist, skip it
                if sp1_wpa < -0.015 || sp2_wpa < -0.015 {
                    return false;
                }
                true
            })
            .map(|(&pair, agg)| {
                // High Bayesian shrinkage (M=40.0) so small-sample flukes never beat proven pairings
                let m_pair = 40.0;
                let wr_score =
                    (agg.wins as f64 + m_pair * champion_win_rate) / (agg.games as f64 + m_pair);
                let strat_score = score_metric(agg, strat_id, champion_win_rate);
                let final_score = if strat_id == "win_rate" {
                    wr_score
                } else {
                    // Blend win rate (60%) with strategy metric (40%) to anchor on reliable performance
                    wr_score * 0.6 + strat_score * 0.4
                };
                (pair, final_score, agg)
            })
            .collect();
        sorted_spells.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // If no pair passed the filter, pick top 2 individual spells by Bayesian credibility
        let chosen_spells = sorted_spells
            .first()
            .map(|(pair, _, _)| *pair)
            .unwrap_or_else(|| {
                let mut top_indiv: Vec<(u64, f64)> = individual_spells_agg
                    .iter()
                    .filter(|(_, (g, _))| *g >= min_pair_games.min(2))
                    .map(|(&sp_id, &(g, w))| {
                        let m = 20.0;
                        let bayes_score = (w as f64 + m * champion_win_rate) / (g as f64 + m);
                        (sp_id, bayes_score)
                    })
                    .collect();
                top_indiv
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                if top_indiv.len() >= 2 {
                    (
                        top_indiv[0].0.min(top_indiv[1].0),
                        top_indiv[0].0.max(top_indiv[1].0),
                    )
                } else {
                    (4, 32) // Flash + Mark fallback
                }
            });

        // E. Select augments (target top 3 silver, top 3 gold, top 3 prismatic)
        let mut silver_augs: Vec<(&String, f64)> = Vec::new();
        let mut gold_augs: Vec<(&String, f64)> = Vec::new();
        let mut prismatic_augs: Vec<(&String, f64)> = Vec::new();
        let mut other_augs: Vec<(&String, f64)> = Vec::new();

        for (aug_id, agg) in &augments_agg {
            if agg.games < min_aug_games && agg.games < 3 {
                continue;
            }
            let score = score_metric(agg, strat_id, champion_win_rate);
            let aug_info = aug_catalog
                .get(aug_id)
                .or_else(|| aug_catalog.get(&aug_id.to_lowercase()));
            match aug_info.and_then(|a| a.rarity.as_deref()) {
                Some("silver") => silver_augs.push((aug_id, score)),
                Some("gold") => gold_augs.push((aug_id, score)),
                Some("prismatic") => prismatic_augs.push((aug_id, score)),
                _ => other_augs.push((aug_id, score)),
            }
        }

        silver_augs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        gold_augs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        prismatic_augs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        other_augs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut chosen_aug_ids = Vec::new();
        for &(id, _) in prismatic_augs.iter().take(3) {
            if !chosen_aug_ids.contains(id) {
                chosen_aug_ids.push(id.clone());
            }
        }
        for &(id, _) in gold_augs.iter().take(3) {
            if !chosen_aug_ids.contains(id) {
                chosen_aug_ids.push(id.clone());
            }
        }
        for &(id, _) in silver_augs.iter().take(3) {
            if !chosen_aug_ids.contains(id) {
                chosen_aug_ids.push(id.clone());
            }
        }
        for (id, _) in &other_augs {
            if chosen_aug_ids.len() >= 9 {
                break;
            }
            if !chosen_aug_ids.contains(id) {
                chosen_aug_ids.push(id.to_string());
            }
        }

        // F. Calculate build WR and WPA
        let core_ids: Vec<i64> = core_items.iter().take(2).map(|(id, _, _)| *id).collect();
        let matching_facts: Vec<&ParticipantFact> = active_facts
            .iter()
            .copied()
            .filter(|f| {
                if core_ids.len() >= 2 {
                    f.item_ids.contains(&core_ids[0]) && f.item_ids.contains(&core_ids[1])
                } else if let Some(&c1) = core_ids.first() {
                    f.item_ids.contains(&c1)
                } else {
                    true
                }
            })
            .collect();

        let (build_win_rate, sample_games) = if matching_facts.len() >= 8 {
            let m_wins = matching_facts.iter().filter(|f| f.win).count();
            (
                m_wins as f64 / matching_facts.len() as f64,
                matching_facts.len(),
            )
        } else {
            // Weighted Bayesian WR of chosen items
            let mut w_sum = 0.0;
            let mut w_count = 0.0;
            for &it_id in &chosen_item_ids {
                if let Some(agg) = items_agg.get(&it_id) {
                    let wr = (agg.wins as f64 + 5.0 * champion_win_rate) / (agg.games as f64 + 5.0);
                    w_sum += wr * (agg.games as f64);
                    w_count += agg.games as f64;
                }
            }
            let fallback_wr = if w_count > 0.0 {
                w_sum / w_count
            } else {
                champion_win_rate
            };
            (fallback_wr, total_games)
        };

        let build_wpa = build_win_rate - champion_win_rate;

        // G. Compute average metric value for strategy
        let (raw_metric_value, avg_metric_value) = match strat_id {
            "win_rate" => (build_win_rate, format!("{:+.1}% WPA", build_wpa * 100.0)),
            "damage_dealt" => {
                let avg = if !matching_facts.is_empty() {
                    matching_facts.iter().map(|f| f.damage).sum::<i64>() as f64
                        / matching_facts.len() as f64
                } else {
                    facts.iter().map(|f| f.damage).sum::<i64>() as f64 / facts.len() as f64
                };
                (avg, format!("{avg:.0} avg dmg"))
            }
            "damage_per_minute" => {
                let avg = if !matching_facts.is_empty() {
                    matching_facts
                        .iter()
                        .map(|f| (f.damage as f64) / (f.game_duration.max(60) as f64 / 60.0))
                        .sum::<f64>()
                        / matching_facts.len() as f64
                } else {
                    facts
                        .iter()
                        .map(|f| (f.damage as f64) / (f.game_duration.max(60) as f64 / 60.0))
                        .sum::<f64>()
                        / facts.len() as f64
                };
                (avg, format!("{avg:.0} DPM"))
            }
            "true_damage" => {
                let avg = if !matching_facts.is_empty() {
                    matching_facts.iter().map(|f| f.true_damage).sum::<i64>() as f64
                        / matching_facts.len() as f64
                } else {
                    facts.iter().map(|f| f.true_damage).sum::<i64>() as f64 / facts.len() as f64
                };
                (avg, format!("{avg:.0} true dmg"))
            }
            "damage_taken" => {
                let avg = if !matching_facts.is_empty() {
                    matching_facts.iter().map(|f| f.damage_taken).sum::<i64>() as f64
                        / matching_facts.len() as f64
                } else {
                    facts.iter().map(|f| f.damage_taken).sum::<i64>() as f64 / facts.len() as f64
                };
                (avg, format!("{avg:.0} dmg taken"))
            }
            "damage_mitigated" => {
                let avg = if !matching_facts.is_empty() {
                    matching_facts
                        .iter()
                        .map(|f| f.damage_mitigated)
                        .sum::<i64>() as f64
                        / matching_facts.len() as f64
                } else {
                    facts.iter().map(|f| f.damage_mitigated).sum::<i64>() as f64
                        / facts.len() as f64
                };
                (avg, format!("{avg:.0} mitigated"))
            }
            "healing" => {
                let avg = if !matching_facts.is_empty() {
                    matching_facts.iter().map(|f| f.total_heal).sum::<i64>() as f64
                        / matching_facts.len() as f64
                } else {
                    facts.iter().map(|f| f.total_heal).sum::<i64>() as f64 / facts.len() as f64
                };
                (avg, format!("{avg:.0} healed"))
            }
            "turret_damage" => {
                let avg = if !matching_facts.is_empty() {
                    matching_facts.iter().map(|f| f.turret_damage).sum::<i64>() as f64
                        / matching_facts.len() as f64
                } else {
                    facts.iter().map(|f| f.turret_damage).sum::<i64>() as f64 / facts.len() as f64
                };
                (avg, format!("{avg:.0} turret dmg"))
            }
            _ => (0.0, String::new()),
        };

        // H. Enrich items with individual WPA and pick rate
        let mut enriched_items = Vec::new();
        for &id in &chosen_item_ids {
            let item_info = items_catalog.get(&(id as i32));
            let is_boot = item_info
                .map(|i| {
                    i.item_class.as_deref() == Some("boot")
                        || i.categories.iter().any(|c| c.eq_ignore_ascii_case("boots"))
                })
                .unwrap_or(false);
            let name = item_info
                .map(|i| i.name.clone())
                .unwrap_or_else(|| format!("Item #{id}"));
            let raw_icon = item_info.map(|i| i.icon_url.clone()).unwrap_or_default();
            let icon_url = get_cached_asset_data_uri(&raw_icon).await;
            let price_total = item_info.map(|i| i.price_total).unwrap_or(0);
            let item_class = item_info.and_then(|i| i.item_class.clone());
            let description = item_info.and_then(|i| i.description.clone());
            let agg = items_agg.get(&id);
            let (wr, games) = agg
                .map(|a| ((a.wins as f64) / (a.games.max(1) as f64), a.games as usize))
                .unwrap_or((champion_win_rate, 0));
            let wpa = wr - champion_win_rate;
            let pick_rate = calc_item_pick_rate(games, total_games, is_boot);
            enriched_items.push(MayhemBuildItem {
                id,
                name,
                icon_url,
                price_total,
                item_class,
                description,
                win_rate: wr,
                wpa,
                games,
                pick_rate,
                situational_tag: get_situational_tag(id, item_info).map(|s| s.to_string()),
            });
        }

        // I. Enrich spells with individual WPA
        let mut enriched_spells = Vec::new();
        for spell_id in [chosen_spells.0, chosen_spells.1] {
            let spell_info = spells_catalog.get(&spell_id);
            let name = spell_info
                .map(|s| s.name.clone())
                .unwrap_or_else(|| format!("Spell #{spell_id}"));
            let raw_icon = spell_info.map(|s| s.icon_url.clone()).unwrap_or_default();
            let icon_url = get_cached_asset_data_uri(&raw_icon).await;
            let description = spell_info.and_then(|s| s.description.clone());
            let (games, wins) = individual_spells_agg
                .get(&spell_id)
                .copied()
                .unwrap_or((0, 0));
            let wr = if games > 0 {
                wins as f64 / games as f64
            } else {
                champion_win_rate
            };
            let wpa = wr - champion_win_rate;
            enriched_spells.push(MayhemBuildSpell {
                id: spell_id,
                name,
                icon_url,
                description,
                win_rate: wr,
                wpa,
                games: games as usize,
            });
        }

        // J. Enrich all spells
        let mut sorted_individual_spells: Vec<(u64, i64, i64, f64, f64)> = individual_spells_agg
            .iter()
            .filter(|(&id, &(games, _))| id > 0 && games >= 1)
            .map(|(&id, &(games, wins))| {
                let wr = wins as f64 / games as f64;
                let wpa = wr - champion_win_rate;
                (id, games, wins, wr, wpa)
            })
            .collect();
        sorted_individual_spells.sort_by(|a, b| b.1.cmp(&a.1));

        let mut enriched_all_spells = Vec::new();
        for (spell_id, games, _, wr, wpa) in sorted_individual_spells {
            let spell_info = spells_catalog.get(&spell_id);
            let name = spell_info
                .map(|s| s.name.clone())
                .unwrap_or_else(|| format!("Spell #{spell_id}"));
            let raw_icon = spell_info.map(|s| s.icon_url.clone()).unwrap_or_default();
            let icon_url = get_cached_asset_data_uri(&raw_icon).await;
            let description = spell_info.and_then(|s| s.description.clone());
            enriched_all_spells.push(MayhemBuildSpell {
                id: spell_id,
                name,
                icon_url,
                description,
                win_rate: wr,
                wpa,
                games: games as usize,
            });
        }

        // K. Enrich augments with individual WPA
        let mut enriched_augments = Vec::new();
        for aug_id in chosen_aug_ids {
            let aug_info = aug_catalog
                .get(&aug_id)
                .or_else(|| aug_catalog.get(&aug_id.to_lowercase()));
            let name = aug_info
                .map(|a| a.name.clone())
                .unwrap_or_else(|| format!("Augment #{aug_id}"));
            let raw_icon = aug_info.map(|a| a.icon_url.clone()).unwrap_or_default();
            let icon_url = get_cached_asset_data_uri(&raw_icon).await;
            let rarity = aug_info.and_then(|a| a.rarity.clone());
            let tooltip = aug_info.and_then(|a| a.tooltip.clone());
            let agg = augments_agg.get(&aug_id);
            let (wr, games) = agg
                .map(|a| ((a.wins as f64) / (a.games.max(1) as f64), a.games as usize))
                .unwrap_or((champion_win_rate, 0));
            let wpa = wr - champion_win_rate;
            let pick_rate = if total_games > 0 {
                games as f64 / total_games as f64
            } else {
                0.0
            };
            enriched_augments.push(MayhemBuildAugment {
                id: aug_id,
                name,
                icon_url,
                rarity,
                tooltip,
                win_rate: wr,
                wpa,
                games,
                pick_rate,
            });
        }

        let mut strat_situational = Vec::new();
        for &(id, _, agg) in &core_items {
            if !chosen_item_ids.contains(&id) {
                if let Some(tag) = get_situational_tag(id, items_catalog.get(&(id as i32))) {
                    let mut it = mayhem_item(
                        id,
                        &items_catalog,
                        Some(agg),
                        champion_win_rate,
                        total_games,
                    )
                    .await;
                    it.situational_tag = Some(tag.to_string());
                    strat_situational.push(it);
                    if strat_situational.len() >= 6 {
                        break;
                    }
                }
            }
        }
        let stages = build_stage_pathing(&starter_item, &enriched_items, &strat_situational);

        strategies.push(MayhemBuildStrategy {
            id: strat_id.to_string(),
            label: strat_label.to_string(),
            metric_name: metric_name.to_string(),
            avg_metric_value,
            raw_metric_value,
            build_win_rate,
            build_wpa,
            starter_item,
            items: enriched_items,
            alternative_items,
            situational_items: strat_situational,
            stages,
            spells: enriched_spells,
            all_spells: enriched_all_spells,
            augments: enriched_augments,
            sample_games,
        });
    }

    // 6. Sort metric strategies by build_wpa; mined archetypes (coherent item cores) go first
    strategies.sort_by(|a, b| {
        b.build_wpa
            .partial_cmp(&a.build_wpa)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let shared_all_spells = strategies
        .first()
        .map(|s| s.all_spells.clone())
        .unwrap_or_default();
    let mut archetype_strategies = Vec::new();
    for arch in mine_build_archetypes(&active_facts, &items_catalog, champion_win_rate) {
        let mut items = Vec::new();
        let mut groups: HashSet<&'static str> = HashSet::new();
        for &id in &arch.core_items {
            if let Some(g) = get_item_exclusive_group(id, items_catalog.get(&(id as i32))) {
                groups.insert(g);
            }
            items.push(
                mayhem_item(
                    id,
                    &items_catalog,
                    items_agg.get(&id),
                    champion_win_rate,
                    total_games,
                )
                .await,
            );
        }
        if let Some(id) = arch.boot_id {
            items.push(
                mayhem_item(
                    id,
                    &items_catalog,
                    items_agg.get(&id),
                    champion_win_rate,
                    total_games,
                )
                .await,
            );
        }
        let mut alternative_items = Vec::new();
        for &(id, games, wr, pr) in arch.continuation_items.iter().take(20) {
            let mut item = mayhem_item(
                id,
                &items_catalog,
                items_agg.get(&id),
                champion_win_rate,
                total_games,
            )
            .await;
            item.games = games;
            item.win_rate = wr;
            item.wpa = wr - champion_win_rate;
            item.pick_rate = pr;
            let grp = get_item_exclusive_group(id, items_catalog.get(&(id as i32)));
            let conflict = grp.map_or(false, |g| groups.contains(g));
            if items.len() < 6 && !conflict {
                if let Some(g) = grp {
                    groups.insert(g);
                }
                items.push(item);
            } else {
                alternative_items.push(item);
            }
        }
        let mut situational_items = Vec::new();
        for &(id, games, wr, pr) in arch.situational_items.iter().take(12) {
            let mut item = mayhem_item(
                id,
                &items_catalog,
                items_agg.get(&id),
                champion_win_rate,
                total_games,
            )
            .await;
            item.games = games;
            item.win_rate = wr;
            item.wpa = wr - champion_win_rate;
            item.pick_rate = pr;
            item.situational_tag =
                get_situational_tag(id, items_catalog.get(&(id as i32))).map(|s| s.to_string());
            situational_items.push(item);
        }
        let starter_item = match arch.starter_id {
            Some(id) => Some(
                mayhem_item(
                    id,
                    &items_catalog,
                    items_agg.get(&id),
                    champion_win_rate,
                    total_games,
                )
                .await,
            ),
            None => None,
        };
        let mut spells = Vec::new();
        for sp in [arch.spells.0, arch.spells.1] {
            spells.push(
                mayhem_spell(
                    sp,
                    &spells_catalog,
                    &individual_spells_agg,
                    champion_win_rate,
                )
                .await,
            );
        }
        let mut augments = Vec::new();
        for (id, games, wr, wpa, pr) in &arch.top_augments {
            if normalized_augments
                .iter()
                .any(|r| r.eq_ignore_ascii_case(id))
            {
                continue;
            }
            let mut aug =
                mayhem_augment(id, &aug_catalog, augments_agg.get(id), champion_win_rate).await;
            aug.games = *games;
            aug.win_rate = *wr;
            aug.wpa = *wpa;
            aug.pick_rate = *pr;
            augments.push(aug);
        }
        if augments.len() < 6 {
            let mut general_augs: Vec<(&String, &MetricAggregate)> = augments_agg
                .iter()
                .filter(|(id, _)| {
                    !augments.iter().any(|a| a.id.eq_ignore_ascii_case(id))
                        && !normalized_augments
                            .iter()
                            .any(|r| r.eq_ignore_ascii_case(id))
                })
                .collect();
            general_augs.sort_by(|a, b| {
                b.1.games
                    .cmp(&a.1.games)
                    .then_with(|| b.1.wins.cmp(&a.1.wins))
            });
            for (id, agg) in general_augs.into_iter().take(6 - augments.len()) {
                let mut aug = mayhem_augment(id, &aug_catalog, Some(agg), champion_win_rate).await;
                if total_games > 0 {
                    aug.pick_rate = agg.games as f64 / total_games as f64;
                }
                augments.push(aug);
            }
        }
        let stages = build_stage_pathing(&starter_item, &items, &situational_items);
        archetype_strategies.push(MayhemBuildStrategy {
            id: arch.id,
            label: arch.label,
            metric_name: "Win Rate".to_string(),
            avg_metric_value: format!("{:+.1}% WPA", arch.build_wpa * 100.0),
            raw_metric_value: arch.build_win_rate,
            build_win_rate: arch.build_win_rate,
            build_wpa: arch.build_wpa,
            starter_item,
            items,
            alternative_items,
            situational_items,
            stages,
            spells,
            all_spells: shared_all_spells.clone(),
            augments,
            sample_games: arch.sample_games,
        });
    }

    for aug in &normalized_augments {
        if let Some(warped_arch) = build_augment_warped_archetype(aug, champion_win_rate) {
            let mut items = Vec::new();
            for &id in &warped_arch.core_items {
                items.push(
                    mayhem_item(
                        id,
                        &items_catalog,
                        items_agg.get(&id),
                        champion_win_rate,
                        total_games,
                    )
                    .await,
                );
            }
            if let Some(id) = warped_arch.boot_id {
                items.push(
                    mayhem_item(
                        id,
                        &items_catalog,
                        items_agg.get(&id),
                        champion_win_rate,
                        total_games,
                    )
                    .await,
                );
            }
            let mut alternative_items = Vec::new();
            for &(id, games, wr, pr) in &warped_arch.continuation_items {
                let mut it = mayhem_item(
                    id,
                    &items_catalog,
                    items_agg.get(&id),
                    champion_win_rate,
                    total_games,
                )
                .await;
                it.games = games;
                it.win_rate = wr;
                it.wpa = wr - champion_win_rate;
                it.pick_rate = pr;
                if items.len() < 6 {
                    items.push(it);
                } else {
                    alternative_items.push(it);
                }
            }
            let mut situational_items = Vec::new();
            for &(id, games, wr, pr) in &warped_arch.situational_items {
                let mut it = mayhem_item(
                    id,
                    &items_catalog,
                    items_agg.get(&id),
                    champion_win_rate,
                    total_games,
                )
                .await;
                it.games = games;
                it.win_rate = wr;
                it.wpa = wr - champion_win_rate;
                it.pick_rate = pr;
                it.situational_tag =
                    get_situational_tag(id, items_catalog.get(&(id as i32))).map(|s| s.to_string());
                situational_items.push(it);
            }
            let starter_item = match warped_arch.starter_id {
                Some(id) => Some(
                    mayhem_item(
                        id,
                        &items_catalog,
                        items_agg.get(&id),
                        champion_win_rate,
                        total_games,
                    )
                    .await,
                ),
                None => None,
            };
            let mut spells = Vec::new();
            for sp in [warped_arch.spells.0, warped_arch.spells.1] {
                spells.push(
                    mayhem_spell(
                        sp,
                        &spells_catalog,
                        &individual_spells_agg,
                        champion_win_rate,
                    )
                    .await,
                );
            }
            let mut augments = Vec::new();
            for (id, games, wr, wpa, pr) in &warped_arch.top_augments {
                let mut a =
                    mayhem_augment(id, &aug_catalog, augments_agg.get(id), champion_win_rate).await;
                a.games = *games;
                a.win_rate = *wr;
                a.wpa = *wpa;
                a.pick_rate = *pr;
                augments.push(a);
            }
            let stages = build_stage_pathing(&starter_item, &items, &situational_items);
            archetype_strategies.insert(
                0,
                MayhemBuildStrategy {
                    id: warped_arch.id,
                    label: warped_arch.label,
                    metric_name: "Win Rate".to_string(),
                    avg_metric_value: format!("{:+.1}% WPA", warped_arch.build_wpa * 100.0),
                    raw_metric_value: warped_arch.build_win_rate,
                    build_win_rate: warped_arch.build_win_rate,
                    build_wpa: warped_arch.build_wpa,
                    starter_item,
                    items,
                    alternative_items,
                    situational_items,
                    stages,
                    spells,
                    all_spells: shared_all_spells.clone(),
                    augments,
                    sample_games: warped_arch.sample_games,
                },
            );
        }
    }
    archetype_strategies.extend(strategies);
    let strategies = archetype_strategies;

    let summary = MayhemChampionBuildSummary {
        champion_id,
        champion_name,
        icon_url,
        champion_win_rate,
        champion_wpa,
        total_games: total_games_all,
        strategies,
        applied_augment_ids,
        fallback_used,
    };

    // 7. Store in cache
    {
        let mut cache = MAYHEM_CHAMPION_BUILDS_CACHE.write().await;
        cache.insert(cache_key, (Instant::now(), summary.clone()));
    }

    Ok(summary)
}

pub fn categorize_augment(name: &str, tooltip: Option<&str>) -> Vec<String> {
    let text = format!("{} {}", name, tooltip.unwrap_or_default()).to_lowercase();
    let mut cats = Vec::new();
    if text.contains("attack damage")
        || text.contains("attack speed")
        || text.contains(" ad ")
        || text.contains("critical")
        || text.contains("on-hit")
        || text.contains("armor penetration")
        || text.contains("lethality")
        || text.contains("scalead")
    {
        cats.push("physical".to_string());
    }
    if text.contains("ability power")
        || text.contains("magic damage")
        || text.contains("scaleap")
        || text.contains("ability haste")
        || text.contains("scaleabilityhaste")
        || text.contains("mana")
        || text.contains("spell")
    {
        cats.push("magic".to_string());
    }
    if text.contains("health")
        || text.contains("scalehealth")
        || text.contains("armor")
        || text.contains("magic resist")
        || text.contains("shield")
        || text.contains("heal")
        || text.contains("damage reduction")
        || text.contains("invulnerable")
    {
        cats.push("defensive".to_string());
    }
    if text.contains("move speed")
        || text.contains("movement speed")
        || text.contains("dash")
        || text.contains("flash")
        || text.contains("snowball")
        || text.contains("haste")
        || text.contains("gold")
        || text.contains("teleport")
    {
        cats.push("utility".to_string());
    }
    if cats.is_empty() {
        cats.push("general".to_string());
    }
    cats
}

pub async fn get_mayhem_augments(
    query: MayhemAugmentsQuery,
) -> Result<MayhemAugmentsResponse, String> {
    let mut with_ids: Vec<String> = query
        .with_augment_ids
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|a| !a.is_empty() && a != "0")
        .collect();
    with_ids.sort();
    with_ids.dedup();

    // 1. Resolve champion data if champion_id is specified
    let (champion_win_rate, champ_total_games, champion_name, champ_augs) =
        if let Some(cid) = query.champion_id {
            if cid > 0 {
                let key = (cid, with_ids.clone());
                // Check cache or compute
                let has_valid_cache = {
                    let cache = MAYHEM_CHAMPION_AUGMENTS_CACHE.read().await;
                    cache
                        .get(&key)
                        .map(|(t, _, _, _)| t.elapsed() < std::time::Duration::from_secs(120))
                        .unwrap_or(false)
                };

                if !has_valid_cache {
                    let _ = compute_mayhem_champion_build(cid, with_ids.clone(), None).await;
                }

                let cache = MAYHEM_CHAMPION_AUGMENTS_CACHE.read().await;
                let (wr, champ_total_games, map) =
                    if let Some((_, c_wr, t_games, aug_map)) = cache.get(&key) {
                        (*c_wr, *t_games, aug_map.clone())
                    } else {
                        (0.50, 0, HashMap::new())
                    };

                let all_champs = get_all_champions().await.unwrap_or_default();
                let c_name = all_champs
                    .iter()
                    .find(|c| c.id == cid)
                    .map(|c| c.name.clone());

                (wr, champ_total_games, c_name, map)
            } else {
                (0.50, 0, None, HashMap::new())
            }
        } else {
            (0.50, 0, None, HashMap::new())
        };

    // 2. Resolve global augment projection cache
    let (global_total_games, global_projection_map) = {
        let cached_map = {
            let cache = MAYHEM_GLOBAL_AUGMENTS_CACHE.read().await;
            cache.as_ref().and_then(|(created_at, total, map)| {
                if created_at.elapsed() < std::time::Duration::from_secs(300) {
                    Some((*total, map.clone()))
                } else {
                    None
                }
            })
        };

        if let Some((total, map)) = cached_map {
            (total, map)
        } else {
            let db_path = history_data_dir().join("global-aram.db");
            let (sample_count, map): (usize, HashMap<String, (usize, f64, f64)>) = if db_path
                .exists()
            {
                tokio::task::spawn_blocking(move || {
                    let mut res = HashMap::new();
                    let mut count = 0usize;
                    if let Ok(connection) = Connection::open(&db_path) {
                        let _ = connection.busy_timeout(std::time::Duration::from_secs(5));
                        let row_opt: Result<(i64, String), _> = connection.query_row(
                            "SELECT sample_count, rows_json FROM statistics_projections WHERE kind = 'augment'",
                            [],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        );
                        if let Ok((sc, json_str)) = row_opt {
                            count = sc.max(0) as usize;
                            if let Ok(rows) = serde_json::from_str::<Vec<Value>>(&json_str) {
                                for r in rows {
                                    let id = r.get("id").and_then(Value::as_str).unwrap_or("");
                                    let clean_id = id.strip_prefix("augment:").unwrap_or(id);
                                    let games = r.get("games").and_then(Value::as_u64).unwrap_or(0)
                                        as usize;
                                    let win_rate =
                                        r.get("win_rate").and_then(Value::as_f64).unwrap_or(0.5);
                                    let wpa = r.get("wpa").and_then(Value::as_f64).unwrap_or(0.0);
                                    res.insert(clean_id.to_string(), (games, win_rate, wpa));
                                    if let Some(name) = r.get("name").and_then(Value::as_str) {
                                        res.insert(name.to_lowercase(), (games, win_rate, wpa));
                                    }
                                }
                            }
                        }
                    }
                    (count, res)
                })
                .await
                .unwrap_or((0, HashMap::new()))
            } else {
                (0, HashMap::new())
            };

            let mut cache = MAYHEM_GLOBAL_AUGMENTS_CACHE.write().await;
            *cache = Some((Instant::now(), sample_count, map.clone()));
            (sample_count, map)
        }
    };

    // 3. Resolve all unique non-null augments from catalog
    let aug_catalog = get_augment_catalog_map().await.unwrap_or_default();
    let mut unique_catalog: HashMap<String, (String, AugmentCatalogEntry)> = HashMap::new();

    for (k, entry) in &aug_catalog {
        let name_low = entry.name.trim().to_lowercase();
        if name_low.is_empty()
            || name_low == "null augment"
            || name_low == "null"
            || entry.tooltip.as_deref().unwrap_or("") == "Null"
        {
            continue;
        }

        if let Some((existing_id, _)) = unique_catalog.get(&name_low) {
            // Prefer purely numeric ID as canonical key
            if !existing_id.chars().all(|c| c.is_ascii_digit())
                && k.chars().all(|c| c.is_ascii_digit())
            {
                unique_catalog.insert(name_low, (k.clone(), entry.clone()));
            }
        } else {
            unique_catalog.insert(name_low, (k.clone(), entry.clone()));
        }
    }

    struct ScoredAugment {
        item: MayhemAugmentItem,
        bayesian_wpa: f64,
        bayesian_wr: f64,
        rarity_rank: u8,
    }

    let mut scored_list = Vec::new();

    for (_name_low, (canonical_id, entry)) in unique_catalog {
        let rarity = entry.rarity.clone();
        let rarity_rank = match rarity.as_deref() {
            Some("prismatic") => 3,
            Some("gold") => 2,
            Some("silver") => 1,
            _ => 0,
        };

        let categories = categorize_augment(&entry.name, entry.tooltip.as_deref());

        // Check global projection statistics
        let global_stat = global_projection_map
            .get(&canonical_id)
            .or_else(|| global_projection_map.get(&entry.name.to_lowercase()))
            .copied();

        let (global_games, global_win_rate, global_wpa) = global_stat.unwrap_or((0, 0.50, 0.0));

        // Augments are offered in 4 slots per game (levels 3, 7, 11, 15) and cannot be repeated.
        // Catalog prior: ~250 augments, so uniform slot prior p0 = 1/250 = 0.004 per slot (1.6% game presence).
        let p0_slot = 0.004f64;
        let m_slots = 20.0f64; // Equivalent to 5 games of 4 slots for Bayesian sample-size shrinkage

        let global_pick_rate = if global_total_games > 0 {
            let total_global_slots = (global_total_games * 4) as f64;
            let slot_share =
                (global_games as f64 + m_slots * p0_slot) / (total_global_slots + m_slots);
            (slot_share * 4.0).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Check champion statistics
        let champ_stat = champ_augs
            .get(&canonical_id)
            .or_else(|| champ_augs.get(&canonical_id.to_lowercase()))
            .or_else(|| champ_augs.get(&entry.name))
            .or_else(|| champ_augs.get(&format!("ARAM_{}", entry.name)));

        // Credibility / Bayesian WPA & WR: Prior weight M = 30.0 with global augment WPA informing the prior
        let prior_wr = (champion_win_rate + global_wpa).clamp(0.10, 0.90);
        let m_wr = 30.0f64;

        let (games, wins, win_rate, wpa, bayesian_wpa, bayesian_wr, pick_rate) =
            if let Some(&(g, w)) = champ_stat {
                let wr = if g > 0 { w as f64 / g as f64 } else { 0.0 };
                let raw_wpa = if g > 0 { wr - champion_win_rate } else { 0.0 };
                let bayes_wr = (w as f64 + m_wr * prior_wr) / (g as f64 + m_wr);
                let bayes_wpa = bayes_wr - champion_win_rate;

                let pr = if champ_total_games > 0 {
                    let total_champ_slots = (champ_total_games * 4) as f64;
                    let slot_share = (g as f64 + m_slots * p0_slot) / (total_champ_slots + m_slots);
                    (slot_share * 4.0).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                (g, w, wr, raw_wpa, bayes_wpa, bayes_wr, pr)
            } else {
                (0, 0, 0.0, 0.0, global_wpa, prior_wr, 0.0)
            };

        let item = MayhemAugmentItem {
            id: canonical_id,
            name: entry.name,
            icon_url: entry.icon_url,
            rarity,
            tooltip: entry.tooltip,
            categories,
            games,
            wins,
            win_rate,
            wpa,
            pick_rate,
            global_games,
            global_win_rate,
            global_wpa,
            global_pick_rate,
        };

        scored_list.push(ScoredAugment {
            item,
            bayesian_wpa,
            bayesian_wr,
            rarity_rank,
        });
    }

    // 4. Filtering
    let tier_filter = query.tier.as_deref().unwrap_or("all").trim().to_lowercase();
    let category_filter = query
        .category
        .as_deref()
        .unwrap_or("all")
        .trim()
        .to_lowercase();
    let search_filter = query.search.as_deref().unwrap_or("").trim().to_lowercase();
    let only_sampled = query.only_sampled.unwrap_or(false);
    let hide_low_pickrate = query.hide_low_pickrate.unwrap_or(false);

    let mut filtered: Vec<ScoredAugment> = scored_list
        .into_iter()
        .filter(|aug| {
            if with_ids.contains(&aug.item.id) {
                return false;
            }
            if tier_filter != "all" && !tier_filter.is_empty() {
                if let Some(r) = aug.item.rarity.as_deref() {
                    if !r.eq_ignore_ascii_case(&tier_filter) {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            if category_filter != "all" && !category_filter.is_empty() {
                if !aug
                    .item
                    .categories
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(&category_filter))
                {
                    return false;
                }
            }

            if only_sampled && aug.item.games == 0 {
                return false;
            }

            if hide_low_pickrate {
                // If hide_low_pickrate is active, filter out augments with low sample / low pick rate (<0.5%)
                if aug.item.games > 0 && aug.item.pick_rate < 0.005 {
                    return false;
                }
            }

            if !search_filter.is_empty() {
                let name_match = aug.item.name.to_lowercase().contains(&search_filter);
                let tooltip_match = aug
                    .item
                    .tooltip
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&search_filter);
                if !name_match && !tooltip_match {
                    return false;
                }
            }

            true
        })
        .collect();

    // 5. Sorting
    let sort_key = query.sort_key.as_deref().unwrap_or("wpa");
    let descending = query.descending.unwrap_or(true);

    filtered.sort_by(|a, b| {
        let ord = match sort_key {
            "wpa" => {
                // If one has champion games and other does not, rank played higher
                if a.item.games > 0 && b.item.games == 0 {
                    std::cmp::Ordering::Greater
                } else if a.item.games == 0 && b.item.games > 0 {
                    std::cmp::Ordering::Less
                } else if a.item.games == 0 && b.item.games == 0 {
                    a.item
                        .global_wpa
                        .partial_cmp(&b.item.global_wpa)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.item.global_games.cmp(&b.item.global_games))
                } else {
                    a.bayesian_wpa
                        .partial_cmp(&b.bayesian_wpa)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.item.games.cmp(&b.item.games))
                        .then_with(|| a.item.global_games.cmp(&b.item.global_games))
                }
            }
            "win_rate" => {
                if a.item.games > 0 && b.item.games == 0 {
                    std::cmp::Ordering::Greater
                } else if a.item.games == 0 && b.item.games > 0 {
                    std::cmp::Ordering::Less
                } else if a.item.games == 0 && b.item.games == 0 {
                    a.item
                        .global_win_rate
                        .partial_cmp(&b.item.global_win_rate)
                        .unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    a.bayesian_wr
                        .partial_cmp(&b.bayesian_wr)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.item.games.cmp(&b.item.games))
                }
            }
            "games" | "sample" => a
                .item
                .games
                .cmp(&b.item.games)
                .then_with(|| {
                    a.item
                        .pick_rate
                        .partial_cmp(&b.item.pick_rate)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.item.global_games.cmp(&b.item.global_games))
                .then_with(|| {
                    a.bayesian_wpa
                        .partial_cmp(&b.bayesian_wpa)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
            "pick_rate" => a
                .item
                .pick_rate
                .partial_cmp(&b.item.pick_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.item.games.cmp(&b.item.games))
                .then_with(|| {
                    a.item
                        .global_pick_rate
                        .partial_cmp(&b.item.global_pick_rate)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
            "name" => a.item.name.to_lowercase().cmp(&b.item.name.to_lowercase()),
            "tier" => a.rarity_rank.cmp(&b.rarity_rank).then_with(|| {
                a.bayesian_wpa
                    .partial_cmp(&b.bayesian_wpa)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            _ => a
                .bayesian_wpa
                .partial_cmp(&b.bayesian_wpa)
                .unwrap_or(std::cmp::Ordering::Equal),
        };

        if descending {
            ord.reverse()
        } else {
            ord
        }
    });

    // 6. Pagination
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);
    let total_items = filtered.len();
    let total_pages = (total_items + page_size - 1) / page_size;
    let page = query.page.unwrap_or(1).clamp(1, total_pages.max(1));
    let start = (page - 1) * page_size;
    let end = (start + page_size).min(total_items);

    let mut result_items = Vec::new();
    if start < total_items {
        for scored in &filtered[start..end] {
            let mut item = scored.item.clone();
            item.icon_url = get_cached_asset_data_uri(&item.icon_url).await;
            result_items.push(item);
        }
    }

    Ok(MayhemAugmentsResponse {
        total_items,
        total_pages,
        page,
        page_size,
        champion_id: query.champion_id,
        champion_name,
        champion_win_rate,
        total_games: champ_total_games,
        items: result_items,
    })
}

fn open_saved_builds_db() -> Result<Connection, String> {
    let dir = history_data_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("failed creating data dir: {e}"))?;
    let conn = Connection::open(dir.join("agent.db"))
        .map_err(|e| format!("failed opening agent.db: {e}"))?;
    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS saved_mayhem_builds (
            id TEXT PRIMARY KEY,
            champion_id INTEGER NOT NULL,
            champion_name TEXT NOT NULL,
            strategy_label TEXT NOT NULL,
            build_json TEXT NOT NULL,
            augment_ids_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_saved_mayhem_builds_champion ON saved_mayhem_builds(champion_id);",
    )
    .map_err(|e| format!("failed creating saved_mayhem_builds: {e}"))?;
    Ok(conn)
}

pub async fn save_mayhem_build(
    payload: SaveMayhemBuildPayload,
) -> Result<SavedMayhemBuild, String> {
    tokio::task::spawn_blocking(move || {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let saved = SavedMayhemBuild {
            id: format!("{}-{nanos}", payload.champion_id),
            champion_id: payload.champion_id,
            champion_name: payload.champion_name,
            strategy_label: payload.strategy_label,
            augment_ids: payload.augment_ids,
            build: payload.build,
            created_at: (nanos / 1_000_000_000).to_string(),
        };
        let conn = open_saved_builds_db()?;
        conn.execute(
            "INSERT INTO saved_mayhem_builds
             (id, champion_id, champion_name, strategy_label, build_json, augment_ids_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                saved.id,
                saved.champion_id,
                saved.champion_name,
                saved.strategy_label,
                serde_json::to_string(&saved.build).map_err(|e| e.to_string())?,
                serde_json::to_string(&saved.augment_ids).map_err(|e| e.to_string())?,
                saved.created_at,
            ],
        )
        .map_err(|e| format!("failed saving build: {e}"))?;
        Ok(saved)
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))?
}

pub async fn list_saved_mayhem_builds(
    champion_id: Option<i64>,
) -> Result<Vec<SavedMayhemBuild>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = open_saved_builds_db()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, champion_id, champion_name, strategy_label, build_json, augment_ids_json, created_at
                 FROM saved_mayhem_builds
                 WHERE (?1 IS NULL OR champion_id = ?1)
                 ORDER BY rowid DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([champion_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        // Rows with unparseable build JSON are skipped, not fatal.
        Ok(rows
            .filter_map(Result::ok)
            .filter_map(|(id, champion_id, champion_name, strategy_label, b, a, created_at)| {
                Some(SavedMayhemBuild {
                    id,
                    champion_id,
                    champion_name,
                    strategy_label,
                    augment_ids: serde_json::from_str(&a).unwrap_or_default(),
                    build: serde_json::from_str(&b).ok()?,
                    created_at,
                })
            })
            .collect())
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))?
}

pub async fn delete_saved_mayhem_build(id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        open_saved_builds_db()?
            .execute("DELETE FROM saved_mayhem_builds WHERE id = ?1", [id])
            .map(|_| ())
            .map_err(|e| format!("failed deleting build: {e}"))
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_situational_items_sorted_by_popularity() {
        let mut items = vec![
            (1001i64, 2usize, 0.60f64, 0.05f64),
            (1002i64, 15usize, 0.45f64, 0.07f64),
            (1003i64, 8usize, 0.80f64, 0.06f64),
            (1004i64, 8usize, 0.90f64, 0.06f64),
        ];

        items.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal))
        });

        assert_eq!(items[0].0, 1002);
        assert_eq!(items[1].0, 1004);
        assert_eq!(items[2].0, 1003);
        assert_eq!(items[3].0, 1001);
    }

    #[test]
    fn test_situational_items_counter_pick_flags() {
        assert_eq!(get_situational_tag(3033, None), Some("Anti-Heal"));
        assert_eq!(get_situational_tag(6609, None), Some("Anti-Heal"));
        assert_eq!(get_situational_tag(3165, None), Some("Anti-Heal"));
        assert_eq!(get_situational_tag(3075, None), Some("Anti-Heal"));
        assert_eq!(get_situational_tag(6695, None), Some("Anti-Shield"));
        assert_eq!(get_situational_tag(4645, None), Some("Anti-Shield"));
        assert_eq!(get_situational_tag(3036, None), Some("Armor Penetration"));
        assert_eq!(get_situational_tag(3135, None), Some("Magic Penetration"));
        assert_eq!(get_situational_tag(3156, None), Some("Magic Resist"));
        assert_eq!(get_situational_tag(3157, None), Some("Armor / Anti-Burst"));
    }

    #[test]
    fn test_marksmage_converts_adc_item_pool_to_ap_on_hit() {
        let augments = vec!["Marksmage".to_string()];
        let bonus = get_item_augment_warp_bonus(3115, &augments);
        assert!(
            bonus > 2.0,
            "Nashor's Tooth must receive high warp bonus under Marksmage"
        );
        let bonus_lich = get_item_augment_warp_bonus(3100, &augments);
        assert!(
            bonus_lich > 2.0,
            "Lich Bane must receive high warp bonus under Marksmage"
        );
        let bonus_rabadon = get_item_augment_warp_bonus(3089, &augments);
        assert!(
            bonus_rabadon > 2.0,
            "Rabadon's must receive high warp bonus under Marksmage"
        );

        let warped = build_augment_warped_archetype("Marksmage", 0.50);
        assert!(
            warped.is_some(),
            "Marksmage warped archetype must be created"
        );
        let arch = warped.unwrap();
        assert!(
            arch.core_items.contains(&3115),
            "Core must contain Nashor's Tooth"
        );
        assert!(
            arch.core_items.contains(&3100),
            "Core must contain Lich Bane"
        );
        assert!(
            arch.core_items.contains(&3089),
            "Core must contain Rabadon's Deathcap"
        );
    }

    #[test]
    fn test_budget_affordability_live_advice() {
        let dummy_strat = MayhemBuildStrategy {
            id: "test".into(),
            label: "Test".into(),
            metric_name: "WR".into(),
            avg_metric_value: "0%".into(),
            raw_metric_value: 0.5,
            build_win_rate: 0.5,
            build_wpa: 0.0,
            starter_item: None,
            items: vec![
                MayhemBuildItem {
                    id: 3006,
                    name: "Berserker's Greaves".into(),
                    icon_url: "".into(),
                    price_total: 1100,
                    item_class: Some("boot".into()),
                    description: None,
                    win_rate: 0.5,
                    wpa: 0.0,
                    games: 100,
                    pick_rate: 0.8,
                    situational_tag: None,
                },
                MayhemBuildItem {
                    id: 3115,
                    name: "Nashor's Tooth".into(),
                    icon_url: "".into(),
                    price_total: 3000,
                    item_class: None,
                    description: None,
                    win_rate: 0.55,
                    wpa: 0.05,
                    games: 100,
                    pick_rate: 0.7,
                    situational_tag: None,
                },
            ],
            alternative_items: vec![],
            situational_items: vec![],
            stages: vec![],
            spells: vec![],
            all_spells: vec![],
            augments: vec![],
            sample_games: 100,
        };

        // Player has 1200 gold and no items
        let advice = compute_live_build_advice(&dummy_strat, 1200.0, &[], 3, 60.0);
        assert_eq!(advice.current_stage, 1);
        assert_eq!(advice.next_target.as_ref().map(|i| i.id), Some(3006));
        assert_eq!(advice.gold_needed, 0);
        assert!(advice.immediate_purchases.iter().any(|i| i.id == 3006));

        // Player owns boots, has 1500 gold
        let advice2 = compute_live_build_advice(&dummy_strat, 1500.0, &[3006], 7, 300.0);
        assert_eq!(advice2.current_stage, 2);
        assert_eq!(advice2.next_target.as_ref().map(|i| i.id), Some(3115));
        assert_eq!(advice2.gold_needed, 1500);
    }

    #[test]
    fn test_v2_infer_rush_items_from_early_matches() {
        let mut facts = Vec::new();
        // Item 6672 (e.g. Kraken Slayer): bought as sole legendary in 5 early games (slot 0)
        for _ in 0..5 {
            facts.push(V2ParsedFact {
                legendaries: vec![(6672, 0)],
                boot: Some(3006),
                starter: Some(1055),
                win: true,
                spell1: 4,
                spell2: 32,
                augments: vec![],
            });
        }
        // 5 games with (6672, 3085)
        for _ in 0..5 {
            facts.push(V2ParsedFact {
                legendaries: vec![(6672, 0), (3085, 1)],
                boot: Some(3006),
                starter: Some(1055),
                win: true,
                spell1: 4,
                spell2: 32,
                augments: vec![],
            });
        }
        // 5 late games finishing 3089 (Rabadon's) in slot 3
        for _ in 0..5 {
            facts.push(V2ParsedFact {
                legendaries: vec![(6672, 0), (3085, 1), (3031, 2), (3089, 3)],
                boot: Some(3006),
                starter: Some(1055),
                win: true,
                spell1: 4,
                spell2: 32,
                augments: vec![],
            });
        }

        let refs: Vec<&V2ParsedFact> = facts.iter().collect();
        let candidates = infer_v2_rush_items_from_refs(&refs, 0.50, 2);

        assert!(!candidates.is_empty(), "Must produce rush candidates");
        assert_eq!(
            candidates[0].item_id, 6672,
            "Item 6672 must be #1 rush item due to solo early completions"
        );
        let rabadon_score = candidates
            .iter()
            .find(|c| c.item_id == 3089)
            .map_or(0.0, |c| c.rush_score);
        assert!(
            candidates[0].rush_score > rabadon_score,
            "Rush item must score higher than late capstone 3089"
        );
    }
}
