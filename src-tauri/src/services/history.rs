use crate::logger;
use crate::models::{
    ChampionSummary, MatchHistoryAugmentInfo, MatchHistoryItem, MatchHistoryItemInfo,
};
use crate::services::asset_cache::get_cached_asset_data_uri;
use crate::services::champions::get_all_champions;
use crate::services::lcu_client::LcuClient;
use futures_util::future::join_all;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
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

fn read_detail_cache(game_id: u64) -> Option<Value> {
    let path = detail_cache_path(game_id);
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

fn write_detail_cache(game_id: u64, response: &Value) {
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

lazy_static! {
    static ref ITEMS_CACHE: RwLock<Option<HashMap<i32, CachedItemInfo>>> = RwLock::new(None);
    static ref MAPS_CACHE: RwLock<Option<HashMap<u16, String>>> = RwLock::new(None);
    static ref AUGMENTS_CATALOG_CACHE: RwLock<Option<HashMap<String, AugmentCatalogEntry>>> =
        RwLock::new(None);
    static ref RESOLVED_AUGMENTS_CACHE: RwLock<Option<HashMap<String, MatchHistoryAugmentInfo>>> =
        RwLock::new(None);
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
                let mut replaced: Option<String> = None;

                if let Some(&base_val) = val_map.get(&p_lower) {
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
                    if let Some((_, calc_obj)) =
                        calcs_map.iter().find(|(k, _)| k.to_lowercase() == p_lower)
                    {
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

pub async fn get_current_summoner_match_history(
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
        if let Some(detail) = read_detail_cache(game_id) {
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
    for batch in uncached_game_ids.chunks(10) {
        let fetched = join_all(
            batch
                .iter()
                .map(|game_id| fetch_game_detail(&client, *game_id)),
        )
        .await;
        for (game_id, detail) in batch.iter().zip(fetched) {
            if let Some(detail) = detail {
                details_by_game_id.insert(*game_id, detail);
            }
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

    for row in rows {
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
    Ok(response)
}
