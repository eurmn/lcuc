use crate::logger;
use crate::models::{
    ChampionSummary, MatchHistoryAugmentInfo, MatchHistoryItem, MatchHistoryItemInfo,
};
use crate::services::champions::get_all_champions;
use crate::services::{asset_cache::get_cached_asset_data_uri, lcu_client::LcuClient};
use futures_util::future::join_all;
use lazy_static::lazy_static;
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
const CDRAGON_AUGMENTS_URL: &str =
    "https://raw.communitydragon.org/latest/game/maps/modespecificdata/kiwi.bin.json";
const CDRAGON_AUGMENT_ICON_BASE: &str =
    "https://raw.communitydragon.org/latest/game/assets/ux/kiwi/augments/icons";
const CDRAGON_CHERRY_AUGMENT_ICON_BASE: &str =
    "https://raw.communitydragon.org/latest/game/assets/ux/cherry/augments/icons";

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
        .join("augments.json")
}

fn read_augment_resolution_cache() -> HashMap<String, MatchHistoryAugmentInfo> {
    let path = augment_resolution_cache_path();
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
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

async fn get_cached_augment_icon(file_name: &str) -> String {
    let kiwi_url = format!("{CDRAGON_AUGMENT_ICON_BASE}/{file_name}");
    let kiwi_icon = get_cached_asset_data_uri(&kiwi_url).await;
    if kiwi_icon != kiwi_url {
        return kiwi_icon;
    }
    let cherry_url = format!("{CDRAGON_CHERRY_AUGMENT_ICON_BASE}/{file_name}");
    get_cached_asset_data_uri(&cherry_url).await
}

lazy_static! {
    static ref ITEMS_CACHE: RwLock<Option<HashMap<i32, (String, String)>>> = RwLock::new(None);
    static ref AUGMENTS_CACHE: RwLock<Option<Value>> = RwLock::new(None);
    static ref RESOLVED_AUGMENTS_CACHE: RwLock<Option<HashMap<String, MatchHistoryAugmentInfo>>> =
        RwLock::new(None);
}

async fn get_items_by_id() -> Result<HashMap<i32, (String, String)>, String> {
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
        items.insert(id, (name, icon_url));
    }

    let mut cache = ITEMS_CACHE.write().await;
    *cache = Some(items.clone());
    Ok(items)
}

async fn get_augment_catalog() -> Result<Value, String> {
    {
        let cache = AUGMENTS_CACHE.read().await;
        if let Some(catalog) = cache.as_ref() {
            return Ok(catalog.clone());
        }
    }
    let catalog: Value = reqwest::Client::new()
        .get(CDRAGON_AUGMENTS_URL)
        .send()
        .await
        .map_err(|error| format!("Failed fetching augment catalog: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Failed parsing augment catalog: {error}"))?;
    let mut cache = AUGMENTS_CACHE.write().await;
    *cache = Some(catalog.clone());
    Ok(catalog)
}

fn find_augment<'a>(value: &'a Value, id: &str) -> Option<&'a Value> {
    match value {
        Value::Object(object) => {
            let matches_id = ["id", "mName", "AugmentNameId", "AugmentPlatformId"]
                .iter()
                .filter_map(|key| object.get(*key))
                .filter_map(value_to_id)
                .any(|candidate| candidate == id);
            if matches_id {
                return Some(value);
            }
            object.values().find_map(|child| find_augment(child, id))
        }
        Value::Array(values) => values.iter().find_map(|child| find_augment(child, id)),
        _ => None,
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
        let Ok(catalog) = get_augment_catalog().await else {
            logger::warn("tauri.history", "Augment catalog unavailable");
            return resolved;
        };
        let new_augments = join_all(unresolved_ids.into_iter().map(|id| async {
            let augment = find_augment(&catalog, &id)?;
            let name = augment
                .get("name")
                .or_else(|| augment.get("mName"))
                .or_else(|| augment.get("AugmentNameId"))
                .and_then(Value::as_str)
                .unwrap_or(&id)
                .to_string();
            let icon_path = augment
                .get("AugmentSmallIconPath")
                .or_else(|| augment.get("AugmentLargeIconPath"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .replace(".tex", ".png");
            if icon_path.is_empty() {
                return None;
            }
            Some(MatchHistoryAugmentInfo {
                id,
                name,
                icon_url: get_cached_augment_icon(
                    icon_path.rsplit('/').next().unwrap_or(&icon_path),
                )
                .await,
            })
        }))
        .await;

        let mut cache = RESOLVED_AUGMENTS_CACHE.write().await;
        let cache = cache.get_or_insert_with(HashMap::new);
        for augment in new_augments.into_iter().flatten() {
            cache.insert(augment.id.clone(), augment.clone());
            resolved.push(augment);
        }
        write_augment_resolution_cache(cache);
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
    logger::debug("tauri.history", "Loading champion metadata");
    let champions_by_id: HashMap<i64, ChampionSummary> = get_all_champions()
        .await?
        .into_iter()
        .map(|champion| (champion.id, champion))
        .collect();
    logger::debug("tauri.history", "Champion metadata ready");
    logger::debug("tauri.history", "Loading item metadata");
    let items_by_id = get_items_by_id().await?;
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
                let item_requests = item_ids.iter().map(|item_id| async {
                    let Some((name, icon_url)) = items_by_id.get(item_id) else {
                        return None;
                    };
                    Some(MatchHistoryItemInfo {
                        id: *item_id,
                        name: name.clone(),
                        icon_url: get_cached_asset_data_uri(icon_url).await,
                    })
                });
                let mut items = Vec::new();
                for (item_id, item) in item_ids.iter().zip(join_all(item_requests).await) {
                    if *item_id == 0 {
                        items.push(None);
                    } else {
                        items.push(item);
                    }
                }

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
        let (name, raw_icon_url) = items.get(&(id as i32))?.clone();
        Some(async move {
            (
                id,
                json!({
                    "id": id,
                    "name": name,
                    "icon_url": get_cached_asset_data_uri(&raw_icon_url).await,
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
            let row_id = row
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| id.starts_with("augment:"))
                .map(|id| id.trim_start_matches("augment:").to_string());
            let partner_ids = row
                .get("partners")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|partner| partner.get("id").and_then(Value::as_str))
                .filter(|id| id.starts_with("augment:"))
                .map(|id| id.trim_start_matches("augment:").to_string());
            row_id.into_iter().chain(partner_ids)
        })
        .map(Value::String)
        .collect::<Vec<_>>();
    let augment_map = resolve_augments(Some(&Value::Array(augment_ids)))
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
            if let Some((name, raw_icon_url)) = items.get(&item_id) {
                object.insert("name".to_string(), Value::String(name.clone()));
                object.insert(
                    "icon_url".to_string(),
                    Value::String(get_cached_asset_data_uri(raw_icon_url).await),
                );
            }
        } else if let Some(augment_id) = id.strip_prefix("augment:") {
            if let Some(augment) = augment_map.get(augment_id) {
                object.insert("name".to_string(), Value::String(augment.name.clone()));
                object.insert(
                    "icon_url".to_string(),
                    Value::String(augment.icon_url.clone()),
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
                    if let Some((name, raw_icon_url)) = items.get(&item_id) {
                        context_object.insert("name".to_string(), Value::String(name.clone()));
                        context_object.insert(
                            "icon_url".to_string(),
                            Value::String(get_cached_asset_data_uri(raw_icon_url).await),
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
                    }
                }
            }
        }
    }
    Ok(response)
}
