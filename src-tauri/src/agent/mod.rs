use crate::logger;
use futures_util::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use native_tls::TlsConnector;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::Connector;

const PORO_SNAX_IDS: [i64; 2] = [2050, 2052];

fn open_connection(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 268435456;
         PRAGMA cache_size = -64000;",
    )?;
    Ok(connection)
}

fn invalidate_statistics_projections(connection: &Connection) -> Result<(), String> {
    connection
        .execute("DELETE FROM statistics_projections", [])
        .map(|_| ())
        .map_err(|error| format!("failed invalidating statistics projections: {error}"))?;
    if let Ok(mut cache) = STATISTICS_QUERY_CACHE.lock() {
        cache.clear();
    }
    if let Ok(mut cache) = PROJECTION_MEMORY_CACHE.lock() {
        cache.clear();
    }
    crate::commands::agent::invalidate_enriched_statistics_cache();
    Ok(())
}

pub fn invalidate_projection_memory_cache() {
    if let Ok(mut cache) = STATISTICS_QUERY_CACHE.lock() {
        cache.clear();
    }
    if let Ok(mut cache) = PROJECTION_MEMORY_CACHE.lock() {
        cache.clear();
    }
    crate::commands::agent::invalidate_enriched_statistics_cache();
}

#[derive(Debug, Deserialize)]
struct AgentRequest {
    token: String,
    command: String,
    #[serde(default)]
    args: Value,
}

#[derive(Debug, Serialize)]
pub struct AgentResponse {
    pub ok: bool,
    pub data: Value,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct RuntimeState {
    pub collector_state: String,
    pub league_client_connected: bool,
    pub queued_jobs: usize,
    pub global_collection_running: bool,
    pub projection_running: bool,
    pub current_patch: Option<String>,
    pub operations: Vec<ProgressOperation>,
    pub crawler_rate_text: String,
    pub crawler_mpm: f64,
    pub crawler_speed_history: std::collections::VecDeque<(std::time::Instant, usize)>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            collector_state: String::new(),
            league_client_connected: false,
            queued_jobs: 0,
            global_collection_running: false,
            projection_running: false,
            current_patch: None,
            operations: Vec::new(),
            crawler_rate_text: "0.0 match/min".to_string(),
            crawler_mpm: 0.0,
            crawler_speed_history: std::collections::VecDeque::new(),
        }
    }
}

impl RuntimeState {
    pub fn update_crawler_progress(&mut self, current_count: usize, live_limit: usize) {
        let now = std::time::Instant::now();
        self.collector_state = format!("global-aram:{current_count}/{live_limit}");
        if let Some(operation) = self
            .operations
            .iter_mut()
            .find(|op| op.id == "global-database")
        {
            operation.current = Some(current_count);
            operation.total = Some(live_limit);
            operation.message = "Updating global game database".to_string();
        }

        self.crawler_speed_history.push_back((now, current_count));
        let cutoff = now.checked_sub(Duration::from_secs(30)).unwrap_or(now);
        while let Some(&(time, _)) = self.crawler_speed_history.front() {
            if time < cutoff && self.crawler_speed_history.len() > 2 {
                self.crawler_speed_history.pop_front();
            } else {
                break;
            }
        }

        if self.crawler_speed_history.len() >= 2 {
            let (oldest_time, oldest_count) = *self.crawler_speed_history.front().unwrap();
            let (newest_time, newest_count) = *self.crawler_speed_history.back().unwrap();
            let time_delta = newest_time.duration_since(oldest_time).as_secs_f64();
            let count_delta = newest_count.saturating_sub(oldest_count);
            if time_delta >= 1.0 {
                let rps = count_delta as f64 / time_delta;
                let mpm = rps * 60.0;
                self.crawler_mpm = mpm;
                if rps >= 1.0 {
                    self.crawler_rate_text = format!("{:.1} match/s", rps);
                } else if mpm >= 1.0 {
                    self.crawler_rate_text = format!("{:.1} match/min", mpm);
                } else {
                    self.crawler_rate_text = format!("{:.0} match/h", mpm * 60.0);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProgressOperation {
    pub id: String,
    pub message: String,
    pub current: Option<usize>,
    pub total: Option<usize>,
}

#[derive(Debug, Clone)]
struct StatisticFact {
    game_id: i64,
    champion_id: Option<i64>,
    win: bool,
    item_ids: Vec<i64>,
    augment_ids: Vec<String>,
    kills: i64,
    deaths: i64,
    assists: i64,
    damage: i64,
    gold: i64,
    duration: i64,
}

#[derive(Debug, Default, Clone)]
struct Aggregate {
    games: i64,
    wins: i64,
    kills: i64,
    deaths: i64,
    assists: i64,
    damage: i64,
    gold: i64,
    duration: i64,
    champion_baseline_sum: f64,
    champions: HashMap<i64, ContextAggregate>,
    partners: HashMap<String, ContextAggregate>,
}

#[derive(Debug, Default, Clone)]
struct ContextAggregate {
    games: i64,
    wins: i64,
}

lazy_static! {
    static ref STATISTICS_QUERY_CACHE: std::sync::Mutex<HashMap<String, (Instant, Value)>> =
        std::sync::Mutex::new(HashMap::new());
    static ref PROJECTION_MEMORY_CACHE: std::sync::Mutex<HashMap<(PathBuf, String), (i64, f64, Vec<Value>)>> =
        std::sync::Mutex::new(HashMap::new());
    static ref COLLECTOR_WAKEUP: tokio::sync::Notify = tokio::sync::Notify::new();
    static ref CRAWLER_GENERATION: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);
}

fn json_i64_array(value: &str) -> Vec<i64> {
    serde_json::from_str::<Vec<Value>>(value)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| item.as_i64())
        .filter(|id| !PORO_SNAX_IDS.contains(id))
        .collect()
}

fn normalize_item_ids(item_ids: Vec<i64>) -> Vec<i64> {
    item_ids
        .into_iter()
        .filter(|id| *id > 0 && !PORO_SNAX_IDS.contains(id))
        .collect()
}

fn remove_poro_snax_from_database(path: &PathBuf, table: &str) -> Result<(), String> {
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening database for item cleanup: {error}"))?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("failed starting item cleanup transaction: {error}"))?;
    let query = format!("SELECT game_id, participant_id, item_ids_json FROM {table}");
    let mut statement = transaction
        .prepare(&query)
        .map_err(|error| format!("failed preparing item cleanup: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| format!("failed reading items for cleanup: {error}"))?;
    let updates = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed collecting items for cleanup: {error}"))?;
    drop(statement);
    let update =
        format!("UPDATE {table} SET item_ids_json = ?3 WHERE game_id = ?1 AND participant_id = ?2");
    for (game_id, participant_id, item_ids_json) in updates {
        let item_ids = normalize_item_ids(json_i64_array(&item_ids_json));
        transaction
            .execute(
                &update,
                rusqlite::params![
                    game_id,
                    participant_id,
                    serde_json::to_string(&item_ids).unwrap_or_else(|_| "[]".to_string())
                ],
            )
            .map_err(|error| format!("failed writing cleaned items: {error}"))?;
    }
    transaction
        .commit()
        .map_err(|error| format!("failed committing item cleanup: {error}"))
}

fn json_string_array(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<Value>>(value)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            item.as_str()
                .map(str::to_string)
                .or_else(|| item.as_i64().map(|id| id.to_string()))
                .or_else(|| {
                    item.get("id")
                        .or_else(|| item.get("augmentId"))
                        .or_else(|| item.get("augmentID"))
                        .or_else(|| item.get("contentId"))
                        .and_then(|id| {
                            id.as_str()
                                .map(str::to_string)
                                .or_else(|| id.as_i64().map(|value| value.to_string()))
                        })
                })
        })
        .filter(|id| !id.is_empty() && id != "0")
        .collect()
}

fn normalize_augment_ids(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.as_str()
                .map(str::to_string)
                .or_else(|| item.as_i64().map(|id| id.to_string()))
                .or_else(|| {
                    item.get("id")
                        .or_else(|| item.get("augmentId"))
                        .or_else(|| item.get("augmentID"))
                        .or_else(|| item.get("contentId"))
                        .and_then(|id| {
                            id.as_str()
                                .map(str::to_string)
                                .or_else(|| id.as_i64().map(|value| value.to_string()))
                        })
                })
        })
        .filter(|id| !id.is_empty() && id != "0")
        .collect()
}

fn participant_augment_ids(participant: &Value) -> Vec<String> {
    let stats = participant.get("stats");
    let mut ids = stats
        .and_then(|value| value.get("augments"))
        .map(normalize_augment_ids)
        .unwrap_or_default();

    for key in [
        "playerAugment1",
        "playerAugment2",
        "playerAugment3",
        "playerAugment4",
        "playerAugment5",
        "playerAugment6",
    ] {
        let value = participant
            .get(key)
            .or_else(|| stats.and_then(|value| value.get(key)));
        if let Some(id) = value.and_then(|value| {
            value
                .as_i64()
                .map(|id| id.to_string())
                .or_else(|| value.as_str().map(str::to_string))
        }) {
            if !id.is_empty() && id != "0" {
                ids.push(id);
            }
        }
    }

    ids.sort();
    ids.dedup();
    ids
}

fn read_statistics_facts(
    connection: &Connection,
    global: bool,
) -> Result<Vec<StatisticFact>, String> {
    let table = if global {
        "aram_participants"
    } else {
        "game_build_facts"
    };
    let games_table = if global { "aram_games" } else { "games" };
    let sql = format!(
        "SELECT p.game_id, p.champion_id, p.win, p.item_ids_json, p.augment_ids_json, p.kills, p.deaths, p.assists, COALESCE(p.damage, 0), p.gold, COALESCE(g.game_duration, 0) FROM {table} p LEFT JOIN {games_table} g ON g.game_id = p.game_id"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("failed preparing statistics facts: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(StatisticFact {
                game_id: row.get(0)?,
                champion_id: row.get(1)?,
                win: row.get::<_, i64>(2)? != 0,
                item_ids: json_i64_array(&row.get::<_, String>(3)?),
                augment_ids: json_string_array(&row.get::<_, String>(4)?),
                kills: row.get(5)?,
                deaths: row.get(6)?,
                assists: row.get(7)?,
                damage: row.get(8)?,
                gold: row.get(9)?,
                duration: row.get(10)?,
            })
        })
        .map_err(|error| format!("failed reading statistics facts: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed collecting statistics facts: {error}"))
}

fn statistic_row(
    id: String,
    name: String,
    kind: &str,
    aggregate: &Aggregate,
    baseline: f64,
    population_count: i64,
) -> Value {
    let games = aggregate.games.max(1) as f64;
    let duration = aggregate.duration.max(1) as f64;
    let champion_baseline = if aggregate.games > 0 {
        aggregate.champion_baseline_sum / aggregate.games as f64
    } else {
        baseline
    };
    let mut champion_rows = aggregate
        .champions
        .iter()
        .map(|(id, context)| {
            let games = context.games.max(1) as f64;
            json!({
                "id": id,
                "name": format!("Champion #{id}"),
                "icon_url": "",
                "games": context.games,
                "wins": context.wins,
                "win_rate": context.wins as f64 / games,
                "wpa": context.wins as f64 / games - baseline,
                "pick_rate": context.games as f64 / (population_count.max(1) as f64),
            })
        })
        .collect::<Vec<_>>();
    champion_rows.sort_by(|a, b| {
        let pr_b = b.get("pick_rate").and_then(Value::as_f64).unwrap_or(0.0);
        let pr_a = a.get("pick_rate").and_then(Value::as_f64).unwrap_or(0.0);
        if (pr_b - pr_a).abs() > f64::EPSILON {
            pr_b.partial_cmp(&pr_a).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            let games_b = b.get("games").and_then(Value::as_i64).unwrap_or(0);
            let games_a = a.get("games").and_then(Value::as_i64).unwrap_or(0);
            games_b.cmp(&games_a)
        }
    });
    champion_rows.truncate(10);
    let mut partner_rows = aggregate
        .partners
        .iter()
        .map(|(id, context)| {
            let games = context.games.max(1) as f64;
            let win_rate = context.wins as f64 / games;
            json!({
                "id": id,
                "name": id,
                "icon_url": "",
                "games": context.games,
                "wins": context.wins,
                "win_rate": win_rate,
                "wpa": win_rate - baseline,
                "pick_rate": context.games as f64 / (population_count.max(1) as f64),
            })
        })
        .collect::<Vec<_>>();
    partner_rows.sort_by(|a, b| {
        let pr_b = b.get("pick_rate").and_then(Value::as_f64).unwrap_or(0.0);
        let pr_a = a.get("pick_rate").and_then(Value::as_f64).unwrap_or(0.0);
        if (pr_b - pr_a).abs() > f64::EPSILON {
            pr_b.partial_cmp(&pr_a).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            let games_b = b.get("games").and_then(Value::as_i64).unwrap_or(0);
            let games_a = a.get("games").and_then(Value::as_i64).unwrap_or(0);
            games_b.cmp(&games_a)
        }
    });
    let mut items = Vec::new();
    let mut augments = Vec::new();
    for p in partner_rows {
        let pid = p.get("id").and_then(Value::as_str).unwrap_or("");
        if pid.starts_with("augment:") {
            if augments.len() < 10 {
                augments.push(p);
            }
        } else if items.len() < 10 {
            items.push(p);
        }
    }
    items.extend(augments);
    let partner_rows = items;
    let effective_baseline = if kind == "champion" || kind == "combo" {
        baseline
    } else {
        champion_baseline
    };
    // Bayesian shrinkage for combos: WR_adj = (wins + M * p0) / (games + M) with M = 10.0
    // ponytail: M=10 prior weight; upgrade to empirical Bayes variance estimation when sample exceeds 100k
    let (win_rate, wpa) = if kind == "combo" {
        let m = 10.0;
        let adj_win_rate = (aggregate.wins as f64 + m * baseline) / (games + m);
        let adj_wpa = adj_win_rate - baseline;
        (adj_win_rate, adj_wpa)
    } else {
        let wr = aggregate.wins as f64 / games;
        (wr, wr - effective_baseline)
    };
    let components = if kind == "combo" {
        id.strip_prefix("combo:")
            .unwrap_or(&id)
            .split('+')
            .map(|part| {
                if let Some(champ_id) = part.strip_prefix("champion:") {
                    json!({
                        "kind": "champion",
                        "id": champ_id,
                        "name": format!("Champion #{champ_id}"),
                        "icon_url": ""
                    })
                } else if let Some(item_id) = part.strip_prefix("item:") {
                    json!({
                        "kind": "item",
                        "id": item_id,
                        "name": format!("Item #{item_id}"),
                        "icon_url": ""
                    })
                } else if let Some(aug_id) = part.strip_prefix("augment:") {
                    json!({
                        "kind": "augment",
                        "id": aug_id,
                        "name": format!("Augment #{aug_id}"),
                        "icon_url": ""
                    })
                } else {
                    json!({
                        "kind": "unknown",
                        "id": part,
                        "name": part,
                        "icon_url": ""
                    })
                }
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut row_json = json!({
        "id": id,
        "name": name,
        "icon_url": "",
        "kind": kind,
        "games": aggregate.games,
        "wins": aggregate.wins,
        "kills": aggregate.kills,
        "deaths": aggregate.deaths,
        "assists": aggregate.assists,
        "damage": aggregate.damage,
        "gold": aggregate.gold,
        "duration": aggregate.duration,
        "win_rate": win_rate,
        "raw_win_rate": aggregate.wins as f64 / games,
        "wpa": wpa,
        "pick_rate": aggregate.games as f64 / (population_count.max(1) as f64),
        "damage_per_minute": aggregate.damage as f64 / (duration / 60.0),
        "champions": champion_rows,
        "partners": partner_rows,
    });
    if kind == "combo" {
        row_json["components"] = Value::Array(components);
    }
    row_json
}

fn load_statistics_projection(
    path: &PathBuf,
    kind: &str,
) -> Result<Option<(i64, f64, Vec<Value>)>, String> {
    if let Ok(cache) = PROJECTION_MEMORY_CACHE.lock() {
        if let Some((sample_count, baseline, rows)) = cache.get(&(path.clone(), kind.to_string())) {
            return Ok(Some((*sample_count, *baseline, rows.clone())));
        }
    }

    let connection = open_connection(path)
        .map_err(|error| format!("failed opening projection database: {error}"))?;
    let row = connection
        .query_row(
            "SELECT sample_count, baseline, rows_json FROM statistics_projections WHERE kind = ?1",
            rusqlite::params![kind],
            |row| {
                let rows_json: String = row.get(2)?;
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    serde_json::from_str::<Vec<Value>>(&rows_json).unwrap_or_default(),
                ))
            },
        )
        .optional()
        .map_err(|error| format!("failed reading statistics projection: {error}"))?;
    if let Some((sample_count, baseline, rows)) = row {
        // Invalidate legacy combo projection that lacks structured components
        if kind == "combo" && rows.iter().any(|r| r.get("components").is_none()) {
            let _ = connection.execute(
                "DELETE FROM statistics_projections WHERE kind = 'combo'",
                [],
            );
            return Ok(None);
        }
        let compacted_rows: Vec<Value> = rows
            .into_iter()
            .map(|r| compact_row_for_projection(&r))
            .collect();
        if let Ok(mut cache) = PROJECTION_MEMORY_CACHE.lock() {
            cache.insert(
                (path.clone(), kind.to_string()),
                (sample_count, baseline, compacted_rows.clone()),
            );
        }
        return Ok(Some((sample_count, baseline, compacted_rows)));
    }
    Ok(None)
}

fn compact_row_for_projection(row: &Value) -> Value {
    let mut compacted = row.clone();
    if let Some(obj) = compacted.as_object_mut() {
        let is_combo = obj.get("kind").and_then(Value::as_str) == Some("combo");
        if let Some(champs) = obj.get_mut("champions").and_then(Value::as_array_mut) {
            let max_champs = if is_combo { 3 } else { 10 };
            if champs.len() > max_champs {
                champs.truncate(max_champs);
            }
        }
        if let Some(partners) = obj.get_mut("partners").and_then(Value::as_array_mut) {
            if is_combo && partners.is_empty() {
                obj.remove("partners");
            } else {
                let mut items = Vec::new();
                let mut augments = Vec::new();
                for p in partners.drain(..) {
                    let pid = p.get("id").and_then(Value::as_str).unwrap_or("");
                    if pid.starts_with("augment:") {
                        if augments.len() < 10 {
                            augments.push(p);
                        }
                    } else if items.len() < 10 {
                        items.push(p);
                    }
                }
                items.extend(augments);
                *partners = items;
            }
        }
    }
    compacted
}

fn store_statistics_projection(
    path: &PathBuf,
    kind: &str,
    sample_count: i64,
    baseline: f64,
    rows: &[Value],
) -> Result<(), String> {
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening projection database: {error}"))?;
    let compacted_rows: Vec<Value> = rows.iter().map(compact_row_for_projection).collect();
    let rows_json = serde_json::to_string(&compacted_rows)
        .map_err(|error| format!("failed encoding {kind} statistics projection: {error}"))?;
    connection
        .execute(
            "INSERT INTO statistics_projections (kind, sample_count, baseline, rows_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)
             ON CONFLICT(kind) DO UPDATE SET
                 sample_count = excluded.sample_count,
                 baseline = excluded.baseline,
                 rows_json = excluded.rows_json,
                 updated_at = CURRENT_TIMESTAMP",
            rusqlite::params![kind, sample_count, baseline, rows_json],
        )
        .map(|_| ())
        .map_err(|error| format!("failed storing {kind} statistics projection: {error}"))?;

    if let Ok(mut cache) = PROJECTION_MEMORY_CACHE.lock() {
        cache.insert(
            (path.clone(), kind.to_string()),
            (sample_count, baseline, compacted_rows),
        );
    }
    crate::commands::agent::invalidate_enriched_statistics_cache();
    Ok(())
}

fn render_statistics_response(
    args: &Value,
    scope: &str,
    kind: &str,
    sample_count: i64,
    baseline: f64,
    mut rows: Vec<Value>,
) -> Value {
    for row in &mut rows {
        if let Some(obj) = row.as_object_mut() {
            if !obj.contains_key("partners") {
                obj.insert("partners".to_string(), Value::Array(Vec::new()));
            }
        }
    }
    let search = args
        .get("search")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let hide_low_pickrate = args
        .get("hide_low_pickrate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let sort_key = args
        .get("sort_key")
        .and_then(Value::as_str)
        .unwrap_or("wpa");
    let descending = args
        .get("descending")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize;
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;

    rows.retain(|row| {
        let matches_search = search.is_empty()
            || row
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains(&search);
        let matches_pick_rate = !hide_low_pickrate
            || row.get("pick_rate").and_then(Value::as_f64).unwrap_or(0.0) >= 0.01;
        matches_search && matches_pick_rate
    });
    rows.sort_by(|left, right| {
        let value = |row: &Value| match sort_key {
            "games" => row.get("games").and_then(Value::as_f64).unwrap_or(0.0),
            "pickRate" => row.get("pick_rate").and_then(Value::as_f64).unwrap_or(0.0),
            "winRate" => row.get("win_rate").and_then(Value::as_f64).unwrap_or(0.0),
            "kda" => {
                let kills = row.get("kills").and_then(Value::as_f64).unwrap_or(0.0);
                let assists = row.get("assists").and_then(Value::as_f64).unwrap_or(0.0);
                let deaths = row.get("deaths").and_then(Value::as_f64).unwrap_or(0.0);
                (kills + assists) / deaths.max(1.0)
            }
            "damagePerMinute" => row
                .get("damage_per_minute")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            "champWpa" => row
                .get("champions")
                .and_then(Value::as_array)
                .and_then(|champs| {
                    champs
                        .iter()
                        .filter_map(|c| c.get("wpa").and_then(Value::as_f64))
                        .fold(None, |acc: Option<f64>, v| {
                            Some(acc.map_or(v, |m| m.max(v)))
                        })
                })
                .unwrap_or(-999.0),
            _ => row.get("wpa").and_then(Value::as_f64).unwrap_or(0.0),
        };
        let ordering = if sort_key == "name" {
            left.get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .cmp(right.get("name").and_then(Value::as_str).unwrap_or(""))
        } else {
            value(left)
                .partial_cmp(&value(right))
                .unwrap_or(std::cmp::Ordering::Equal)
        };
        if descending {
            ordering.reverse()
        } else {
            ordering
        }
    });
    let total = rows.len();
    json!({
        "scope": scope,
        "kind": kind,
        "sample_count": sample_count,
        "baseline": baseline,
        "total": total,
        "offset": offset,
        "limit": limit,
        "rows": rows.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),
        "cached": true
    })
}

fn build_statistics_response(
    args: &Value,
    database: &PathBuf,
    global_database: &PathBuf,
) -> Result<Value, String> {
    let raw_scope = args
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("personal");
    let kind = args
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("champion");
    // ponytail: combos require global ARAM dataset; personal sample is too small for tuple mining
    let scope = if kind == "combo" && raw_scope == "personal" {
        "global"
    } else {
        raw_scope
    };
    let search = args
        .get("search")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let hide_low_pickrate = args
        .get("hide_low_pickrate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let sort_key = args
        .get("sort_key")
        .and_then(Value::as_str)
        .unwrap_or("wpa");
    let descending = args
        .get("descending")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize;
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;

    let mut cache_args = args.clone();
    if let Some(object) = cache_args.as_object_mut() {
        object.remove("offset");
        object.remove("limit");
    }
    let cache_key = cache_args.to_string();
    if let Ok(mut cache) = STATISTICS_QUERY_CACHE.lock() {
        if let Some((created_at, cached)) = cache.get(&cache_key) {
            if created_at.elapsed() < Duration::from_secs(60) {
                let cached_rows = cached
                    .get("rows")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let total = cached_rows.len();
                let rows = cached_rows
                    .into_iter()
                    .skip(offset)
                    .take(limit)
                    .collect::<Vec<_>>();
                let mut resp = cached.clone();
                if let Some(obj) = resp.as_object_mut() {
                    obj.insert("total".to_string(), Value::from(total));
                    obj.insert("offset".to_string(), Value::from(offset));
                    obj.insert("limit".to_string(), Value::from(limit));
                    obj.insert("rows".to_string(), Value::Array(rows));
                    obj.insert("cached".to_string(), Value::Bool(true));
                }
                return Ok(resp);
            }
        }
        cache.retain(|_, (created_at, _)| created_at.elapsed() < Duration::from_secs(180));
    }

    if scope == "compare" {
        let mut personal_args = args.clone();
        let mut global_args = args.clone();
        personal_args["scope"] = Value::String("personal".to_string());
        global_args["scope"] = Value::String("global".to_string());
        personal_args["offset"] = Value::from(0);
        global_args["offset"] = Value::from(0);
        personal_args["limit"] = Value::from(10000);
        global_args["limit"] = Value::from(10000);
        personal_args["search"] = Value::String(String::new());
        global_args["search"] = Value::String(String::new());
        personal_args["hide_low_pickrate"] = Value::Bool(false);
        global_args["hide_low_pickrate"] = Value::Bool(false);
        personal_args["sort_key"] = Value::String("name".to_string());
        global_args["sort_key"] = Value::String("name".to_string());
        personal_args["descending"] = Value::Bool(false);
        global_args["descending"] = Value::Bool(false);
        let personal = build_statistics_response(&personal_args, database, global_database)?;
        let global = build_statistics_response(&global_args, database, global_database)?;
        let personal_rows = personal
            .get("rows")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let global_rows = global
            .get("rows")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let personal_by_id = personal_rows
            .into_iter()
            .filter_map(|row| {
                let id = row.get("id").and_then(Value::as_str).map(str::to_string)?;
                Some((id, row))
            })
            .collect::<HashMap<_, _>>();
        let mut rows_by_id = global_rows
            .into_iter()
            .filter_map(|row| {
                let id = row.get("id").and_then(Value::as_str).map(str::to_string)?;
                Some((id, row))
            })
            .collect::<HashMap<_, _>>();
        let global_ids = rows_by_id.keys().cloned().collect::<HashSet<_>>();
        for (id, row) in &personal_by_id {
            rows_by_id.entry(id.clone()).or_insert_with(|| row.clone());
        }
        let mut rows = rows_by_id
            .into_iter()
            .map(|(id, mut row)| {
                let personal = personal_by_id.get(&id);
                let global_observed = global_ids.contains(&id);
                let global_win_rate = if global_observed {
                    row.get("win_rate").and_then(Value::as_f64).unwrap_or(0.0)
                } else {
                    0.0
                };
                let global_wpa = if global_observed {
                    row.get("wpa").and_then(Value::as_f64).unwrap_or(0.0)
                } else {
                    0.0
                };
                let global_games = if global_observed {
                    row.get("games").cloned().unwrap_or(Value::from(0))
                } else {
                    Value::from(0)
                };
                let personal_games = personal
                    .and_then(|item| item.get("games"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                let personal_win_rate = personal
                    .and_then(|item| item.get("win_rate"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let personal_wpa = if personal.is_some() && global_observed {
                    personal_win_rate - global_win_rate
                } else {
                    0.0
                };
                if let Some(object) = row.as_object_mut() {
                    object.insert("global_observed".into(), Value::from(global_observed));
                    object.insert("personal_games".into(), Value::from(personal_games));
                    object.insert("personal_win_rate".into(), Value::from(personal_win_rate));
                    object.insert("personal_wpa".into(), Value::from(personal_wpa));
                    object.insert("global_games".into(), global_games);
                    object.insert("global_win_rate".into(), Value::from(global_win_rate));
                    object.insert("global_wpa".into(), Value::from(global_wpa));
                    object.insert(
                        "personal_wpa_vs_global_win_rate".into(),
                        Value::from(personal_wpa),
                    );
                    if !object.contains_key("components") {
                        if let Some(comps) = personal.and_then(|p| p.get("components")) {
                            object.insert("components".into(), comps.clone());
                        }
                    }
                }
                row
            })
            .collect::<Vec<_>>();
        rows.retain(|row| {
            let matches_search = search.is_empty()
                || row
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains(&search);
            let matches_pick_rate = !hide_low_pickrate
                || row.get("pick_rate").and_then(Value::as_f64).unwrap_or(0.0) >= 0.01;
            matches_search && matches_pick_rate
        });
        rows.sort_by(|left, right| {
            let value = |row: &Value| match sort_key {
                "games" => row
                    .get("global_games")
                    .or_else(|| row.get("games"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                "pickRate" => row.get("pick_rate").and_then(Value::as_f64).unwrap_or(0.0),
                "winRate" => row
                    .get("global_win_rate")
                    .or_else(|| row.get("win_rate"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                "personalWinRate" => row
                    .get("personal_win_rate")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                "globalWinRate" => row
                    .get("global_win_rate")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                "kda" => {
                    let kills = row.get("kills").and_then(Value::as_f64).unwrap_or(0.0);
                    let assists = row.get("assists").and_then(Value::as_f64).unwrap_or(0.0);
                    let deaths = row.get("deaths").and_then(Value::as_f64).unwrap_or(0.0);
                    (kills + assists) / deaths.max(1.0)
                }
                "personalWpa" => row
                    .get("personal_wpa")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                "globalWpa" => row.get("global_wpa").and_then(Value::as_f64).unwrap_or(0.0),
                "champWpa" => row
                    .get("champions")
                    .and_then(Value::as_array)
                    .and_then(|champs| {
                        champs
                            .iter()
                            .filter_map(|c| c.get("wpa").and_then(Value::as_f64))
                            .fold(None, |acc: Option<f64>, v| {
                                Some(acc.map_or(v, |m| m.max(v)))
                            })
                    })
                    .unwrap_or(-999.0),
                _ => row
                    .get("global_wpa")
                    .or_else(|| row.get("wpa"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
            };
            let ordering = if sort_key == "name" {
                left.get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(right.get("name").and_then(Value::as_str).unwrap_or(""))
            } else {
                value(left)
                    .partial_cmp(&value(right))
                    .unwrap_or(std::cmp::Ordering::Equal)
            };
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
        let total = rows.len();
        let cached_rows = rows.clone();
        if let Ok(mut cache) = STATISTICS_QUERY_CACHE.lock() {
            cache.insert(
                cache_key,
                (
                    Instant::now(),
                    json!({
                        "scope": "compare",
                        "kind": kind,
                        "sample_count": global.get("sample_count").and_then(Value::as_i64).unwrap_or(0),
                        "baseline": global.get("baseline").and_then(Value::as_f64).unwrap_or(0.0),
                        "personal_sample_count": personal.get("sample_count").and_then(Value::as_i64).unwrap_or(0),
                        "personal_baseline": personal.get("baseline").and_then(Value::as_f64).unwrap_or(0.0),
                        "global_sample_count": global.get("sample_count").and_then(Value::as_i64).unwrap_or(0),
                        "global_baseline": global.get("baseline").and_then(Value::as_f64).unwrap_or(0.0),
                        "rows": cached_rows
                    }),
                ),
            );
        }
        return Ok(json!({
            "scope": "compare",
            "kind": kind,
            "sample_count": global.get("sample_count").and_then(Value::as_i64).unwrap_or(0),
            "baseline": global.get("baseline").and_then(Value::as_f64).unwrap_or(0.0),
            "personal_sample_count": personal.get("sample_count").and_then(Value::as_i64).unwrap_or(0),
            "personal_baseline": personal.get("baseline").and_then(Value::as_f64).unwrap_or(0.0),
            "global_sample_count": global.get("sample_count").and_then(Value::as_i64).unwrap_or(0),
            "global_baseline": global.get("baseline").and_then(Value::as_f64).unwrap_or(0.0),
            "total": total,
            "offset": offset,
            "limit": limit,
            "rows": rows.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),
            "cached": false
        }));
    }
    let projection_path = if scope == "global" {
        global_database
    } else {
        database
    };
    if let Some((sample_count, baseline, rows)) = load_statistics_projection(projection_path, kind)?
    {
        return Ok(render_statistics_response(
            args,
            scope,
            kind,
            sample_count,
            baseline,
            rows,
        ));
    }
    let mut facts = Vec::new();
    if scope == "personal" || scope == "both" {
        let connection = open_connection(database)
            .map_err(|error| format!("failed opening statistics database: {error}"))?;
        facts.extend(read_statistics_facts(&connection, false)?);
    }
    if scope == "global" || scope == "both" {
        let connection = open_connection(global_database)
            .map_err(|error| format!("failed opening global statistics database: {error}"))?;
        facts.extend(read_statistics_facts(&connection, true)?);
    }

    let sample_count = facts
        .iter()
        .map(|fact| fact.game_id)
        .collect::<HashSet<_>>()
        .len() as i64;
    let population_count = facts.len() as i64;
    let mut champion_totals = HashMap::<i64, (i64, i64)>::new();
    for fact in &facts {
        if let Some(champion_id) = fact.champion_id {
            let entry = champion_totals.entry(champion_id).or_default();
            entry.0 += 1;
            entry.1 += i64::from(fact.win);
        }
    }
    let mut baseline_games = 0i64;
    let mut baseline_wins = 0i64;
    for fact in &facts {
        let observed = match kind {
            "item" => !fact.item_ids.is_empty(),
            "augment" => !fact.augment_ids.is_empty(),
            "combo" => {
                let active_count = (if fact.champion_id.is_some() { 1 } else { 0 })
                    + fact
                        .item_ids
                        .iter()
                        .filter(|id| !PORO_SNAX_IDS.contains(id) && **id > 0)
                        .count()
                    + fact
                        .augment_ids
                        .iter()
                        .filter(|id| !id.is_empty() && *id != "0")
                        .count();
                active_count >= 2
            }
            _ => fact.champion_id.is_some(),
        };
        if observed {
            baseline_games += 1;
            baseline_wins += i64::from(fact.win);
        }
    }
    let baseline = if baseline_games > 0 {
        baseline_wins as f64 / baseline_games as f64
    } else {
        0.0
    };
    let min_combo_games = if population_count > 500 {
        3
    } else if population_count > 50 {
        2
    } else {
        1
    };

    // Apriori optimization: pre-calculate candidate frequent components and pairs
    let (combo_frequent_pairs, combo_fact_components) = if kind == "combo" {
        // Step 1: count individual components (champion, item, augment) across facts
        let mut comp_counts: HashMap<String, i64> = HashMap::new();
        for fact in &facts {
            if let Some(champ_id) = fact.champion_id {
                *comp_counts
                    .entry(format!("champion:{champ_id}"))
                    .or_default() += 1;
            }
            for &id in &fact.item_ids {
                if !PORO_SNAX_IDS.contains(&id) && id > 0 {
                    *comp_counts.entry(format!("item:{id}")).or_default() += 1;
                }
            }
            for id in &fact.augment_ids {
                if !id.is_empty() && *id != "0" {
                    *comp_counts.entry(format!("augment:{id}")).or_default() += 1;
                }
            }
        }
        // Min support for single component: 2% pick rate floor, clamped by min_combo_games
        let min_component_support =
            ((population_count as f64 * 0.02).ceil() as i64).max(min_combo_games);
        let frequent_components: HashSet<String> = comp_counts
            .into_iter()
            .filter(|(_, count)| *count >= min_component_support)
            .map(|(id, _)| id)
            .collect();

        // Step 2: filter fact components down to frequent items/augments
        let mut filtered_fact_comps = Vec::with_capacity(facts.len());
        let mut pair_counts: HashMap<(String, String), i64> = HashMap::new();
        for fact in &facts {
            let mut components = Vec::new();
            if let Some(champ_id) = fact.champion_id {
                let key = format!("champion:{champ_id}");
                if frequent_components.contains(&key) {
                    components.push(key);
                }
            }
            let mut items = fact
                .item_ids
                .iter()
                .filter(|id| !PORO_SNAX_IDS.contains(id) && **id > 0)
                .copied()
                .collect::<Vec<_>>();
            items.sort_unstable();
            items.dedup();
            for id in items {
                let key = format!("item:{id}");
                if frequent_components.contains(&key) {
                    components.push(key);
                }
            }
            let mut augs = fact
                .augment_ids
                .iter()
                .filter(|id| !id.is_empty() && *id != "0")
                .cloned()
                .collect::<Vec<_>>();
            augs.sort();
            augs.dedup();
            for id in augs {
                let key = format!("augment:{id}");
                if frequent_components.contains(&key) {
                    components.push(key);
                }
            }
            components.sort();

            // Count 2-tuples for frequent pair detection
            let len = components.len();
            for i in 0..len {
                for j in (i + 1)..len {
                    *pair_counts
                        .entry((components[i].clone(), components[j].clone()))
                        .or_default() += 1;
                }
            }
            filtered_fact_comps.push(components);
        }

        let frequent_pairs: HashSet<(String, String)> = pair_counts
            .into_iter()
            .filter(|(_, count)| *count >= min_combo_games)
            .map(|(pair, _)| pair)
            .collect();

        (frequent_pairs, filtered_fact_comps)
    } else {
        (HashSet::new(), Vec::new())
    };

    let mut aggregates: HashMap<String, (String, Aggregate)> = HashMap::new();
    for (fact_idx, fact) in facts.iter().enumerate() {
        let keys = match kind {
            "item" => fact
                .item_ids
                .iter()
                .filter(|id| !PORO_SNAX_IDS.contains(id))
                .map(|id| (format!("item:{id}"), format!("Item #{id}")))
                .collect::<Vec<_>>(),
            "augment" => fact
                .augment_ids
                .iter()
                .map(|id| (format!("augment:{id}"), format!("Augment #{id}")))
                .collect::<Vec<_>>(),
            "combo" => {
                let components = &combo_fact_components[fact_idx];
                let mut tuples = Vec::new();
                let len = components.len();
                // k = 2 combinations: only frequent pairs
                for i in 0..len {
                    for j in (i + 1)..len {
                        let pair = (&components[i], &components[j]);
                        if combo_frequent_pairs.contains(&(pair.0.clone(), pair.1.clone())) {
                            let id = format!("combo:{}+{}", pair.0, pair.1);
                            let name = format!("{} + {}", pair.0, pair.1);
                            tuples.push((id, name));
                        }
                    }
                }
                // k = 3 combinations: Apriori principle (all 3 constituent pairs must be frequent)
                for i in 0..len {
                    for j in (i + 1)..len {
                        if !combo_frequent_pairs
                            .contains(&(components[i].clone(), components[j].clone()))
                        {
                            continue;
                        }
                        for l in (j + 1)..len {
                            if combo_frequent_pairs
                                .contains(&(components[i].clone(), components[l].clone()))
                                && combo_frequent_pairs
                                    .contains(&(components[j].clone(), components[l].clone()))
                            {
                                let id = format!(
                                    "combo:{}+{}+{}",
                                    components[i], components[j], components[l]
                                );
                                let name = format!(
                                    "{} + {} + {}",
                                    components[i], components[j], components[l]
                                );
                                tuples.push((id, name));
                            }
                        }
                    }
                }
                tuples
            }
            _ => fact
                .champion_id
                .map(|id| vec![(format!("champion:{id}"), format!("Champion #{id}"))])
                .unwrap_or_default(),
        };
        for (id, name) in keys {
            let entry = aggregates
                .entry(id.clone())
                .or_insert_with(|| (name, Aggregate::default()));
            let aggregate = &mut entry.1;
            aggregate.games += 1;
            aggregate.wins += i64::from(fact.win);
            aggregate.kills += fact.kills;
            aggregate.deaths += fact.deaths;
            aggregate.assists += fact.assists;
            aggregate.damage += fact.damage;
            aggregate.gold += fact.gold;
            aggregate.duration += fact.duration;
            if let Some(champion_id) = fact.champion_id {
                if let Some((champion_games, champion_wins)) = champion_totals.get(&champion_id) {
                    aggregate.champion_baseline_sum +=
                        *champion_wins as f64 / (*champion_games).max(1) as f64;
                }
                let context = aggregate.champions.entry(champion_id).or_default();
                context.games += 1;
                context.wins += i64::from(fact.win);
            }
            if kind != "combo" {
                for item_id in &fact.item_ids {
                    let partner = format!("item:{item_id}");
                    if partner != id {
                        let context = aggregate.partners.entry(partner).or_default();
                        context.games += 1;
                        context.wins += i64::from(fact.win);
                    }
                }
                for augment_id in &fact.augment_ids {
                    let partner = format!("augment:{augment_id}");
                    if partner != id {
                        let context = aggregate.partners.entry(partner).or_default();
                        context.games += 1;
                        context.wins += i64::from(fact.win);
                    }
                }
            }
        }
    }
    let min_combo_games = if population_count > 500 {
        3
    } else if population_count > 50 {
        2
    } else {
        1
    };
    let mut rows = aggregates
        .into_iter()
        .filter_map(|(id, (name, aggregate))| {
            if kind == "combo" && aggregate.games < min_combo_games {
                return None;
            }
            let row = statistic_row(id, name, kind, &aggregate, baseline, population_count);
            if search.is_empty()
                || row
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains(&search)
            {
                Some(row)
            } else {
                None
            }
        })
        .filter(|row| {
            !hide_low_pickrate
                || row.get("pick_rate").and_then(Value::as_f64).unwrap_or(0.0) >= 0.01
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let value = |row: &Value| match sort_key {
            "games" => row.get("games").and_then(Value::as_f64).unwrap_or(0.0),
            "pickRate" => row.get("pick_rate").and_then(Value::as_f64).unwrap_or(0.0),
            "winRate" => row.get("win_rate").and_then(Value::as_f64).unwrap_or(0.0),
            "kda" => {
                let kills = row.get("kills").and_then(Value::as_f64).unwrap_or(0.0);
                let assists = row.get("assists").and_then(Value::as_f64).unwrap_or(0.0);
                let deaths = row.get("deaths").and_then(Value::as_f64).unwrap_or(0.0);
                (kills + assists) / deaths.max(1.0)
            }
            "damagePerMinute" => row
                .get("damage_per_minute")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            "champWpa" => row
                .get("champions")
                .and_then(Value::as_array)
                .and_then(|champs| {
                    champs
                        .iter()
                        .filter_map(|c| c.get("wpa").and_then(Value::as_f64))
                        .fold(None, |acc: Option<f64>, v| {
                            Some(acc.map_or(v, |m| m.max(v)))
                        })
                })
                .unwrap_or(-999.0),
            _ => row.get("wpa").and_then(Value::as_f64).unwrap_or(0.0),
        };
        let ordering = if sort_key == "name" {
            let left_name = left.get("name").and_then(Value::as_str).unwrap_or("");
            let right_name = right.get("name").and_then(Value::as_str).unwrap_or("");
            left_name.cmp(right_name)
        } else {
            value(left)
                .partial_cmp(&value(right))
                .unwrap_or(std::cmp::Ordering::Equal)
        };
        if descending {
            ordering.reverse()
        } else {
            ordering
        }
    });
    if search.is_empty() && !hide_low_pickrate && sort_key == "name" {
        let projection_rows = if kind == "combo" && rows.len() > 1000 {
            // ponytail: cap projection to top 1000 combos; increase limit if memory allows
            let mut top_rows = rows.clone();
            top_rows.sort_by(|a, b| {
                let wpa_b = b.get("wpa").and_then(Value::as_f64).unwrap_or(0.0);
                let wpa_a = a.get("wpa").and_then(Value::as_f64).unwrap_or(0.0);
                wpa_b
                    .partial_cmp(&wpa_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            top_rows.truncate(1000);
            top_rows.sort_by(|a, b| {
                let name_a = a.get("name").and_then(Value::as_str).unwrap_or("");
                let name_b = b.get("name").and_then(Value::as_str).unwrap_or("");
                name_a.cmp(name_b)
            });
            top_rows
        } else {
            rows.clone()
        };
        store_statistics_projection(
            if scope == "global" {
                global_database
            } else {
                database
            },
            kind,
            sample_count,
            baseline,
            &projection_rows,
        )?;
    }
    let total = rows.len();
    let cached_rows = rows.clone();
    if let Ok(mut cache) = STATISTICS_QUERY_CACHE.lock() {
        cache.insert(
            cache_key,
            (
                Instant::now(),
                json!({
                    "sample_count": sample_count,
                    "baseline": baseline,
                    "rows": cached_rows
                }),
            ),
        );
    }
    let rows = rows
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    Ok(
        json!({ "scope": scope, "kind": kind, "sample_count": sample_count, "baseline": baseline, "total": total, "offset": offset, "limit": limit, "rows": rows, "cached": false }),
    )
}

fn compact_stored_projections(path: &PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening database for projection compaction: {error}"))?;
    let mut statement = match connection
        .prepare("SELECT kind, sample_count, baseline, rows_json FROM statistics_projections")
    {
        Ok(stmt) => stmt,
        Err(_) => return Ok(()),
    };
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("failed querying projections for compaction: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("failed collecting projections for compaction: {e}"))?;
    drop(statement);

    let mut compacted_any = false;
    for (kind, _sample_count, _baseline, rows_json) in rows {
        if rows_json.len() > 500_000 {
            if let Ok(parsed_rows) = serde_json::from_str::<Vec<Value>>(&rows_json) {
                let compacted: Vec<Value> = parsed_rows
                    .into_iter()
                    .map(|r| compact_row_for_projection(&r))
                    .collect();
                if let Ok(new_json) = serde_json::to_string(&compacted) {
                    let _ = connection.execute(
                        "UPDATE statistics_projections SET rows_json = ?1, updated_at = CURRENT_TIMESTAMP WHERE kind = ?2",
                        rusqlite::params![new_json, kind],
                    );
                    compacted_any = true;
                    logger::info(
                        "agent.database",
                        &format!(
                            "Compacted {kind} projection in {:?}: {} bytes -> {} bytes",
                            path.file_name().unwrap_or_default(),
                            rows_json.len(),
                            new_json.len()
                        ),
                    );
                }
            }
        }
    }
    if compacted_any {
        let _ = connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }
    Ok(())
}

pub fn database_path() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu-companion")
        .join("agent.db")
}

pub fn global_aram_database_path() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu-companion")
        .join("global-aram.db")
}

fn initialize_global_aram_database(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed creating global ARAM database directory: {error}"))?;
    }
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global ARAM database: {error}"))?;
    connection
        .execute_batch(
            r#"
        PRAGMA journal_mode = WAL;
        PRAGMA busy_timeout = 5000;
        CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS discovered_accounts (
            puuid TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'queued',
            discovered_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS aram_games (
            game_id INTEGER PRIMARY KEY,
            source_puuid TEXT,
            queue_id INTEGER,
            map_id INTEGER,
            game_mode TEXT NOT NULL,
            game_creation INTEGER,
            game_duration INTEGER,
            game_version TEXT,
            patch TEXT,
            detail_json TEXT NOT NULL,
            discovered_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS aram_participants (
            game_id INTEGER NOT NULL,
            participant_id INTEGER NOT NULL,
            puuid TEXT,
            champion_id INTEGER,
            win INTEGER NOT NULL DEFAULT 0,
            item_ids_json TEXT NOT NULL DEFAULT '[]',
            augment_ids_json TEXT NOT NULL DEFAULT '[]',
            kills INTEGER NOT NULL DEFAULT 0,
            deaths INTEGER NOT NULL DEFAULT 0,
            assists INTEGER NOT NULL DEFAULT 0,
            damage INTEGER,
            gold INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (game_id, participant_id)
        );
        CREATE TABLE IF NOT EXISTS statistics_projections (
            kind TEXT PRIMARY KEY,
            sample_count INTEGER NOT NULL,
            baseline REAL NOT NULL,
            rows_json TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_aram_participants_puuid ON aram_participants(puuid);
        CREATE INDEX IF NOT EXISTS idx_aram_participants_champion ON aram_participants(champion_id);
        CREATE INDEX IF NOT EXISTS idx_aram_participants_game ON aram_participants(game_id);
        CREATE INDEX IF NOT EXISTS idx_discovered_accounts_status ON discovered_accounts(status);
        INSERT OR IGNORE INTO config (key, value) VALUES ('game_limit', '1000');
        INSERT OR IGNORE INTO config (key, value) VALUES ('worker_count', '3');
        UPDATE config SET value = '3' WHERE key = 'worker_count';
        DELETE FROM config WHERE key = 'rate_limit_games_ceiling_rpm' AND CAST(value AS INTEGER) < 120;
        DELETE FROM config WHERE key = 'rate_limit_games_rpm' AND CAST(value AS INTEGER) < 120;
        "#,
        )
        .map_err(|error| format!("failed initializing global ARAM database: {error}"))?;
    let _ = connection.execute("ALTER TABLE aram_games ADD COLUMN game_version TEXT", []);
    let _ = connection.execute("ALTER TABLE aram_games ADD COLUMN patch TEXT", []);
    let _ = connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_aram_games_patch ON aram_games(patch)",
        [],
    );
    let cleanup_needed = connection
        .query_row(
            "SELECT value FROM config WHERE key = 'poro_snax_cleaned_v3'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("failed reading Poro-Snax cleanup state: {error}"))?
        .is_none();
    drop(connection);
    if cleanup_needed {
        remove_poro_snax_from_database(path, "aram_participants")?;
        let connection = open_connection(path)
            .map_err(|error| format!("failed reopening global ARAM database: {error}"))?;
        connection
            .execute(
                "INSERT INTO config (key, value, updated_at) VALUES ('poro_snax_cleaned_v3', '1', CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
                [],
            )
            .map_err(|error| format!("failed recording Poro-Snax cleanup state: {error}"))?;
    }
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global database for wpa migration: {error}"))?;
    let wpa_fix_needed = connection
        .query_row(
            "SELECT value FROM config WHERE key = 'champion_wpa_fixed_v1'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("failed reading global WPA fix state: {error}"))?
        .is_none();
    if wpa_fix_needed {
        let _ = connection.execute("DELETE FROM statistics_projections", []);
        let _ = connection.execute(
            "INSERT INTO config (key, value, updated_at) VALUES ('champion_wpa_fixed_v1', '1', CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            [],
        );
        if let Ok(mut cache) = STATISTICS_QUERY_CACHE.lock() {
            cache.clear();
        }
    }
    drop(connection);
    repair_global_augment_ids(path)?;
    backfill_global_game_patches(path)?;
    compact_stored_projections(path)?;
    Ok(())
}

fn backfill_global_game_patches(path: &PathBuf) -> Result<(), String> {
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global database for patch backfill: {error}"))?;
    let backfilled = connection
        .query_row(
            "SELECT value FROM config WHERE key = 'game_patches_backfilled_v1'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("failed reading patch backfill state: {error}"))?
        .is_some();
    if backfilled {
        return Ok(());
    }

    let mut statement = connection
        .prepare("SELECT game_id, detail_json FROM aram_games WHERE patch IS NULL OR patch = ''")
        .map_err(|error| format!("failed preparing patch backfill: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("failed reading games for patch backfill: {error}"))?;
    let details = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed collecting games for patch backfill: {error}"))?;
    drop(statement);

    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("failed starting patch backfill transaction: {error}"))?;
    for (game_id, detail_json) in details {
        let detail = serde_json::from_str::<Value>(&detail_json).unwrap_or(Value::Null);
        let game_version = detail
            .get("gameVersion")
            .and_then(Value::as_str)
            .map(str::to_string);
        let patch = game_version
            .as_deref()
            .and_then(crate::services::patch_manager::normalize_patch);
        if game_version.is_some() || patch.is_some() {
            transaction
                .execute(
                    "UPDATE aram_games SET game_version = ?2, patch = ?3 WHERE game_id = ?1",
                    rusqlite::params![game_id, game_version, patch],
                )
                .map_err(|error| {
                    format!("failed backfilling patch for global game {game_id}: {error}")
                })?;
        }
    }
    transaction
        .execute(
            "INSERT INTO config (key, value, updated_at) VALUES ('game_patches_backfilled_v1', '1', CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            [],
        )
        .map_err(|error| format!("failed recording patch backfill state: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("failed committing patch backfill: {error}"))
}

fn repair_global_augment_ids(path: &PathBuf) -> Result<(), String> {
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global database for augment repair: {error}"))?;
    let repaired = connection
        .query_row(
            "SELECT value FROM config WHERE key = 'augment_ids_normalized_v1'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("failed reading augment repair state: {error}"))?
        .is_some();
    if repaired {
        return Ok(());
    }

    let mut statement = connection
        .prepare("SELECT game_id, detail_json FROM aram_games")
        .map_err(|error| format!("failed preparing augment repair: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("failed reading games for augment repair: {error}"))?;
    let details = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed collecting games for augment repair: {error}"))?;
    drop(statement);

    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("failed starting augment repair transaction: {error}"))?;
    for (game_id, detail_json) in details {
        let detail = serde_json::from_str::<Value>(&detail_json).unwrap_or(Value::Null);
        if let Some(participants) = detail.get("participants").and_then(Value::as_array) {
            for participant in participants {
                let participant_id = participant.get("participantId").and_then(Value::as_i64);
                let augment_ids = participant_augment_ids(participant);
                if let (Some(participant_id), false) = (participant_id, augment_ids.is_empty()) {
                    transaction
                        .execute(
                            "UPDATE aram_participants SET augment_ids_json = ?3
                             WHERE game_id = ?1 AND participant_id = ?2",
                            rusqlite::params![
                                game_id,
                                participant_id,
                                serde_json::to_string(&augment_ids)
                                    .unwrap_or_else(|_| "[]".to_string())
                            ],
                        )
                        .map_err(|error| {
                            format!(
                                "failed repairing augments for global participant {game_id}:{participant_id}: {error}"
                            )
                        })?;
                }
            }
        }
    }
    transaction
        .execute(
            "INSERT INTO config (key, value, updated_at) VALUES ('augment_ids_normalized_v1', '1', CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            [],
        )
        .map_err(|error| format!("failed recording augment repair state: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("failed committing augment repair: {error}"))
}

fn legacy_database_path() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu_companion")
        .join("lcu_companion.db")
}

pub fn initialize_database(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed creating agent database directory: {error}"))?;
    }

    let connection =
        open_connection(path).map_err(|error| format!("Failed opening agent database: {error}"))?;

    connection
        .execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS sync_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS accounts (
                puuid TEXT PRIMARY KEY,
                summoner_name TEXT,
                game_name TEXT,
                tag_line TEXT,
                platform_id TEXT,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS games (
                game_id INTEGER PRIMARY KEY,
                puuid TEXT,
                queue_id INTEGER,
                map_id INTEGER,
                game_mode TEXT,
                game_creation INTEGER,
                game_duration INTEGER,
                game_version TEXT,
                patch TEXT,
                collection_status TEXT NOT NULL DEFAULT 'discovered',
                discovered_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS game_build_facts (
                game_id INTEGER NOT NULL,
                participant_id INTEGER NOT NULL,
                puuid TEXT,
                champion_id INTEGER,
                win INTEGER NOT NULL DEFAULT 0,
                item_ids_json TEXT NOT NULL DEFAULT '[]',
                augment_ids_json TEXT NOT NULL DEFAULT '[]',
                kills INTEGER NOT NULL DEFAULT 0,
                deaths INTEGER NOT NULL DEFAULT 0,
                assists INTEGER NOT NULL DEFAULT 0,
                damage INTEGER,
                gold INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (game_id, participant_id)
            );

            CREATE INDEX IF NOT EXISTS idx_game_build_facts_champion ON game_build_facts(champion_id);
            CREATE INDEX IF NOT EXISTS idx_game_build_facts_game ON game_build_facts(game_id);
            CREATE INDEX IF NOT EXISTS idx_game_build_facts_puuid ON game_build_facts(puuid);

            CREATE TABLE IF NOT EXISTS statistics_projections (
                kind TEXT PRIMARY KEY,
                sample_count INTEGER NOT NULL,
                baseline REAL NOT NULL,
                rows_json TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS collection_errors (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER,
                route TEXT NOT NULL,
                error TEXT NOT NULL,
                retryable INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS runesets (
                id INTEGER PRIMARY KEY,
                summoner_puuid TEXT,
                champion_id INTEGER,
                game_id INTEGER,
                rune_page_id INTEGER,
                rune_page_name TEXT,
                primary_style_id INTEGER,
                sub_style_id INTEGER,
                selected_perk_ids_json TEXT NOT NULL,
                source TEXT NOT NULL,
                role INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS favorite_champions (
                champion_id INTEGER PRIMARY KEY,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS legacy_imports (
                source_path TEXT PRIMARY KEY,
                imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            INSERT OR IGNORE INTO schema_migrations (version) VALUES (1);
            "#,
        )
        .map_err(|error| format!("Failed initializing agent database schema: {error}"))?;
    let _ = connection.execute("ALTER TABLE games ADD COLUMN game_version TEXT", []);
    let _ = connection.execute("ALTER TABLE games ADD COLUMN patch TEXT", []);
    let _ = connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_games_patch ON games(patch)",
        [],
    );
    import_legacy_store(&connection, path)?;
    let cleanup_needed = connection
        .query_row(
            "SELECT value FROM sync_state WHERE key = 'poro_snax_cleaned_v3'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("failed reading Poro-Snax cleanup state: {error}"))?
        .is_none();
    let wpa_fix_needed = connection
        .query_row(
            "SELECT value FROM sync_state WHERE key = 'champion_wpa_fixed_v1'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("failed reading champion WPA fix state: {error}"))?
        .is_none();
    if wpa_fix_needed {
        let _ = connection.execute("DELETE FROM statistics_projections", []);
        let _ = connection.execute(
            "INSERT INTO sync_state (key, value, updated_at) VALUES ('champion_wpa_fixed_v1', '1', CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            [],
        );
        if let Ok(mut cache) = STATISTICS_QUERY_CACHE.lock() {
            cache.clear();
        }
    }
    drop(connection);
    if cleanup_needed {
        remove_poro_snax_from_database(path, "game_build_facts")?;
        let connection = open_connection(path)
            .map_err(|error| format!("failed reopening agent database: {error}"))?;
        connection
            .execute(
                "INSERT INTO sync_state (key, value, updated_at) VALUES ('poro_snax_cleaned_v3', '1', CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
                [],
            )
            .map_err(|error| format!("failed recording Poro-Snax cleanup state: {error}"))?;
    }
    compact_stored_projections(path)?;
    Ok(())
}

fn import_legacy_store(connection: &Connection, agent_path: &PathBuf) -> Result<(), String> {
    let legacy_path = legacy_database_path();
    if !legacy_path.exists() || legacy_path == *agent_path {
        return Ok(());
    }

    let source_key = legacy_path.to_string_lossy().to_string();
    let already_imported: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM legacy_imports WHERE source_path = ?1)",
            rusqlite::params![source_key],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed checking legacy import state: {error}"))?;
    if already_imported {
        return Ok(());
    }

    connection
        .execute(
            "ATTACH DATABASE ?1 AS legacy",
            rusqlite::params![legacy_path.to_string_lossy().to_string()],
        )
        .map_err(|error| format!("failed attaching legacy database: {error}"))?;

    let result = (|| {
        let has_role: bool = connection
            .prepare("PRAGMA legacy.table_info(runesets)")
            .and_then(|mut statement| {
                let mut rows = statement.query([])?;
                while let Some(row) = rows.next()? {
                    let name: String = row.get(1)?;
                    if name == "role" {
                        return Ok(true);
                    }
                }
                Ok(false)
            })
            .unwrap_or(false);

        let runeset_columns = if has_role {
            "id, summoner_puuid, champion_id, game_id, rune_page_id, rune_page_name,
             primary_style_id, sub_style_id, selected_perk_ids_json, source, role, created_at"
        } else {
            "id, summoner_puuid, champion_id, game_id, rune_page_id, rune_page_name,
             primary_style_id, sub_style_id, selected_perk_ids_json, source, 1, created_at"
        };

        connection
            .execute(
                &format!(
                    "INSERT OR IGNORE INTO runesets (
                        id, summoner_puuid, champion_id, game_id, rune_page_id,
                        rune_page_name, primary_style_id, sub_style_id,
                        selected_perk_ids_json, source, role, created_at
                    ) SELECT {runeset_columns} FROM legacy.runesets"
                ),
                [],
            )
            .map_err(|error| format!("failed importing legacy runesets: {error}"))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO favorite_champions (champion_id, created_at)
                 SELECT champion_id, created_at FROM legacy.favorite_champions",
                [],
            )
            .map_err(|error| format!("failed importing favorite champions: {error}"))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO app_settings (key, value, updated_at)
                 SELECT key, value, updated_at FROM legacy.app_settings",
                [],
            )
            .map_err(|error| format!("failed importing app settings: {error}"))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO legacy_imports (source_path) VALUES (?1)",
                rusqlite::params![source_key],
            )
            .map_err(|error| format!("failed recording legacy import: {error}"))?;
        Ok(())
    })();

    let detach_result = connection
        .execute("DETACH DATABASE legacy", [])
        .map(|_| ())
        .map_err(|error| format!("failed detaching legacy database: {error}"));
    result.and(detach_result)
}

fn response_ok(data: Value) -> AgentResponse {
    AgentResponse {
        ok: true,
        data,
        error: None,
    }
}

fn response_error(error: impl Into<String>) -> AgentResponse {
    AgentResponse {
        ok: false,
        data: Value::Null,
        error: Some(error.into()),
    }
}

async fn handle_request(
    request: AgentRequest,
    expected_token: &str,
    path: &PathBuf,
    runtime: &Arc<RwLock<RuntimeState>>,
) -> AgentResponse {
    if request.token != expected_token {
        return response_error("unauthorized");
    }

    handle_command(&request.command, request.args, path, runtime).await
}

pub async fn handle_command(
    command: &str,
    args: Value,
    path: &PathBuf,
    runtime: &Arc<RwLock<RuntimeState>>,
) -> AgentResponse {
    match command {
        "health" => {
            let database_ready = path.exists();
            let state = runtime.read().await;
            response_ok(json!({
                "service": "lcu-companion-agent",
                "database_ready": database_ready,
                "collector_state": state.collector_state.clone(),
                "protocol_version": 1,
            }))
        }
        "state" => {
            let state = runtime.read().await;
            response_ok(json!({
                "collector_state": state.collector_state.clone(),
                "league_client_connected": state.league_client_connected,
                "queued_jobs": state.queued_jobs,
                "operations": state.operations.iter().map(|operation| json!({
                    "id": operation.id,
                    "message": operation.message,
                    "current": operation.current,
                    "total": operation.total
                })).collect::<Vec<_>>()
            }))
        }
        "build_statistics" => {
            // ponytail: offload synchronous rusqlite read to blocking pool
            let database_path = path.clone();
            let result = tokio::task::spawn_blocking(move || {
                let connection = open_connection(&database_path)
                    .map_err(|error| format!("failed opening agent database: {error}"))?;
                let mut statement = connection.prepare(
                    "SELECT game_id, participant_id, puuid, champion_id, win,
                            item_ids_json, augment_ids_json, kills, deaths, assists, damage, gold
                     FROM game_build_facts ORDER BY game_id DESC LIMIT 500",
                ).map_err(|error| format!("failed preparing build statistics: {error}"))?;
                let rows = statement.query_map([], |row| {
                    Ok(json!({
                        "game_id": row.get::<_, i64>(0)?,
                        "participant_id": row.get::<_, i64>(1)?,
                        "puuid": row.get::<_, Option<String>>(2)?,
                        "champion_id": row.get::<_, Option<i64>>(3)?,
                        "win": row.get::<_, i64>(4)? != 0,
                        "item_ids": serde_json::from_str::<Value>(&row.get::<_, String>(5)?).unwrap_or_else(|_| json!([])),
                        "augment_ids": serde_json::from_str::<Value>(&row.get::<_, String>(6)?).unwrap_or_else(|_| json!([])),
                        "kills": row.get::<_, i64>(7)?,
                        "deaths": row.get::<_, i64>(8)?,
                        "assists": row.get::<_, i64>(9)?,
                        "damage": row.get::<_, Option<i64>>(10)?,
                        "gold": row.get::<_, i64>(11)?,
                    }))
                }).map_err(|error| format!("failed reading build statistics: {error}"))?;
                let facts: Result<Vec<Value>, _> = rows.collect();
                facts.map_err(|error| format!("failed collecting build statistics: {error}"))
            })
            .await;
            match result {
                Ok(Ok(facts)) => response_ok(json!({ "facts": facts })),
                Ok(Err(error)) => response_error(error),
                Err(error) => response_error(format!("build statistics worker failed: {error}")),
            }
        }
        "statistics" => {
            let started = Instant::now();
            let global_path = global_aram_database_path();
            let args = args.clone();
            let database = path.clone();
            match tokio::task::spawn_blocking(move || {
                initialize_global_aram_database(&global_path)?;
                build_statistics_response(&args, &database, &global_path)
            })
            .await
            {
                Ok(Ok(data)) => {
                    logger::debug(
                        "agent.statistics",
                        &format!(
                            "Prepared statistics response in {}ms",
                            started.elapsed().as_millis()
                        ),
                    );
                    response_ok(data)
                }
                Ok(Err(error)) => response_error(error),
                Err(error) => response_error(format!("statistics worker failed: {error}")),
            }
        }
        "global_aram_config" => {
            let global_path = global_aram_database_path();
            let config_path = global_path.clone();
            let config = tokio::task::spawn_blocking(move || {
                initialize_global_aram_database(&config_path)?;
                Ok::<_, String>((
                    global_game_limit(&config_path)?,
                    global_game_count(&config_path)?,
                    global_queued_account_count(&config_path).unwrap_or(0),
                    global_worker_count(&config_path).unwrap_or(3),
                    global_patch_breakdown(&config_path).unwrap_or_default(),
                ))
            })
            .await;
            let (limit, games, queued_accounts, _workers, patches) = match config {
                Ok(Ok(values)) => values,
                Ok(Err(error)) => return response_error(error),
                Err(error) => {
                    return response_error(format!("global config worker failed: {error}"))
                }
            };
            let state = runtime.read().await;
            let current_patch = state
                .current_patch
                .clone()
                .or_else(crate::services::patch_manager::get_cached_patch);
            let outdated_games: usize = if let Some(ref cp) = current_patch {
                patches
                    .iter()
                    .filter(|(p, _)| p != cp && p != "unknown")
                    .map(|(_, c)| *c)
                    .sum()
            } else {
                // If active patch is not yet cached, check if DB itself spans multiple known patches
                let mut known_patches: Vec<(String, usize)> = patches
                    .iter()
                    .filter(|(p, _)| p != "unknown")
                    .cloned()
                    .collect();
                if known_patches.len() > 1 {
                    known_patches.sort_by(|(a, _), (b, _)| {
                        let va = crate::services::patch_manager::parse_patch_numbers(a)
                            .unwrap_or((0, 0));
                        let vb = crate::services::patch_manager::parse_patch_numbers(b)
                            .unwrap_or((0, 0));
                        vb.cmp(&va)
                    });
                    known_patches.into_iter().skip(1).map(|(_, c)| c).sum()
                } else {
                    0
                }
            };
            let has_patch_mismatch = outdated_games > 0;
            response_ok(json!({
                "limit": limit,
                "games": games,
                "workers": 4,
                "database_path": global_path,
                "collector_state": state.collector_state,
                "queued_jobs": queued_accounts,
                "current_patch": current_patch,
                "patches": patches.iter().map(|(patch, count)| json!({
                    "patch": patch,
                    "games": count
                })).collect::<Vec<_>>(),
                "outdated_games": outdated_games,
                "has_patch_mismatch": has_patch_mismatch,
                "crawler_rate_text": state.crawler_rate_text,
                "crawler_mpm": state.crawler_mpm,
                "operations": state.operations.iter().map(|operation| json!({
                    "id": operation.id,
                    "message": operation.message,
                    "current": operation.current,
                    "total": operation.total
                })).collect::<Vec<_>>()
            }))
        }
        "reset_global_aram" => {
            CRAWLER_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let global_path = global_aram_database_path();
            let agent_path = path.clone();
            let reset_path = global_path.clone();
            let result = tokio::task::spawn_blocking(move || {
                initialize_global_aram_database(&reset_path)?;
                let connection = open_connection(&reset_path).map_err(|error| {
                    format!("failed opening global ARAM database for reset: {error}")
                })?;
                connection
                    .execute_batch(
                        "DELETE FROM aram_games;
                         DELETE FROM aram_participants;
                         DELETE FROM discovered_accounts;
                         DELETE FROM statistics_projections;",
                    )
                    .map_err(|error| format!("failed clearing global ARAM tables: {error}"))?;
                let _ = connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
                drop(connection);

                // Re-seed BFS search tree root with the current summoner PUUID if known
                if let Ok(Some(seed_puuid)) =
                    sync_state_value(&agent_path, "match_history_backfill_v2_puuid")
                {
                    if !seed_puuid.trim().is_empty() {
                        let _ = queue_global_account(&reset_path, seed_puuid.trim());
                    }
                }
                if let Ok(agent_conn) = open_connection(&agent_path) {
                    let _ = invalidate_statistics_projections(&agent_conn);
                } else if let Ok(mut cache) = STATISTICS_QUERY_CACHE.lock() {
                    cache.clear();
                }

                Ok::<_, String>((
                    global_game_limit(&reset_path)?,
                    global_worker_count(&reset_path).unwrap_or(3),
                    global_queued_account_count(&reset_path).unwrap_or(0),
                ))
            })
            .await;

            match result {
                Ok(Ok((limit, workers, queued_accounts))) => {
                    {
                        let mut state = runtime.write().await;
                        state.global_collection_running = false;
                        state.operations.retain(|operation| {
                            operation.id != "global-database" && operation.id != "global-statistics"
                        });
                        state.collector_state = format!("global-aram:0/{limit} (reset)");
                    }
                    COLLECTOR_WAKEUP.notify_one();
                    logger::info(
                        "agent.global-aram",
                        "Reset global ARAM Mayhem database and BFS search tree",
                    );
                    response_ok(json!({
                        "reset": true,
                        "limit": limit,
                        "games": 0,
                        "workers": workers,
                        "queued_jobs": queued_accounts,
                    }))
                }
                Ok(Err(error)) => response_error(error),
                Err(error) => response_error(format!("reset global ARAM worker failed: {error}")),
            }
        }
        "set_global_aram_limit" => {
            let global_path = global_aram_database_path();
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(1000) as usize;
            let path_clone = global_path.clone();
            let result = tokio::task::spawn_blocking(move || {
                initialize_global_aram_database(&path_clone)?;
                set_global_game_limit(&path_clone, limit)?;
                global_game_count(&path_clone)
            })
            .await;
            match result {
                Ok(Ok(games)) => {
                    let mut state = runtime.write().await;
                    state.collector_state = if games >= limit {
                        format!("global-aram:{games}/{limit} (paused)")
                    } else {
                        format!("global-aram:{games}/{limit} (starting)")
                    };
                    response_ok(json!({ "limit": limit }))
                }
                Ok(Err(error)) => response_error(error),
                Err(error) => response_error(format!("set limit worker failed: {error}")),
            }
        }
        "set_global_aram_workers" => {
            let global_path = global_aram_database_path();
            let workers = args.get("workers").and_then(Value::as_u64).unwrap_or(3) as usize;
            let path_clone = global_path.clone();
            let result = tokio::task::spawn_blocking(move || {
                initialize_global_aram_database(&path_clone)?;
                set_global_worker_count(&path_clone, workers)
            })
            .await;
            match result {
                Ok(Ok(())) => response_ok(json!({ "workers": workers.clamp(1, 16) })),
                Ok(Err(error)) => response_error(error),
                Err(error) => response_error(format!("set workers worker failed: {error}")),
            }
        }
        "global_aram_statistics" => {
            let global_path = global_aram_database_path();
            let result = tokio::task::spawn_blocking(move || {
                initialize_global_aram_database(&global_path)?;
                let connection = open_connection(&global_path)
                    .map_err(|error| format!("failed opening global ARAM database: {error}"))?;
                let mut statement = connection
                    .prepare(
                        "SELECT game_id, participant_id, puuid, champion_id, win,
                                item_ids_json, augment_ids_json, kills, deaths, assists, damage, gold
                         FROM aram_participants ORDER BY game_id DESC LIMIT 100000",
                    )
                    .map_err(|error| {
                        format!("failed preparing global ARAM statistics: {error}")
                    })?;
                let rows = statement
                    .query_map([], |row| {
                        Ok(json!({
                            "game_id": row.get::<_, i64>(0)?,
                            "participant_id": row.get::<_, i64>(1)?,
                            "puuid": row.get::<_, Option<String>>(2)?,
                            "champion_id": row.get::<_, Option<i64>>(3)?,
                            "win": row.get::<_, i64>(4)? != 0,
                            "item_ids": serde_json::from_str::<Value>(&row.get::<_, String>(5)?).unwrap_or_else(|_| json!([])),
                            "augment_ids": serde_json::from_str::<Value>(&row.get::<_, String>(6)?).unwrap_or_else(|_| json!([])),
                            "kills": row.get::<_, i64>(7)?,
                            "deaths": row.get::<_, i64>(8)?,
                            "assists": row.get::<_, i64>(9)?,
                            "damage": row.get::<_, Option<i64>>(10)?,
                            "gold": row.get::<_, i64>(11)?,
                        }))
                    })
                    .map_err(|error| {
                        format!("failed reading global ARAM statistics: {error}")
                    })?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|error| {
                        format!("failed collecting global ARAM statistics: {error}")
                    })
            })
            .await;
            match result {
                Ok(Ok(facts)) => response_ok(json!({ "facts": facts })),
                Ok(Err(error)) => response_error(error),
                Err(error) => response_error(format!("global statistics worker failed: {error}")),
            }
        }
        "shutdown" => response_ok(json!({ "accepted": true })),
        command => response_error(format!("unknown command: {command}")),
    }
}

pub async fn dispatch_command(
    command: &str,
    args: Value,
    path: &PathBuf,
    runtime: &Arc<RwLock<RuntimeState>>,
) -> Result<Value, String> {
    let response = handle_command(command, args, path, runtime).await;
    if response.ok {
        Ok(response.data)
    } else {
        Err(response
            .error
            .unwrap_or_else(|| "agent request failed".to_string()))
    }
}

fn store_recent_games(
    path: &PathBuf,
    raw_history: &Value,
    puuid: Option<&str>,
) -> Result<(usize, bool), String> {
    let games = raw_history
        .get("games")
        .and_then(|value| value.get("games"))
        .and_then(Value::as_array)
        .ok_or_else(|| "match history response did not contain games.games".to_string())?;
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening agent database for games: {error}"))?;
    let game_ids = games
        .iter()
        .filter_map(|game| game.get("gameId").and_then(Value::as_i64))
        .collect::<Vec<_>>();
    let new_game_discovered = game_ids.iter().any(|game_id| {
        connection
            .query_row(
                "SELECT 1 FROM games WHERE game_id = ?1 LIMIT 1",
                rusqlite::params![game_id],
                |_| Ok(()),
            )
            .is_err()
    });
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("failed starting games transaction: {error}"))?;
    for game in games {
        let Some(game_id) = game.get("gameId").and_then(Value::as_i64) else {
            continue;
        };
        let game_version = game.get("gameVersion").and_then(Value::as_str);
        let patch = game_version.and_then(crate::services::patch_manager::normalize_patch);
        transaction
            .execute(
                "INSERT INTO games (
                    game_id, puuid, queue_id, map_id, game_mode,
                    game_creation, game_duration, game_version, patch, collection_status, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'discovered', CURRENT_TIMESTAMP)
                ON CONFLICT(game_id) DO UPDATE SET
                    puuid = COALESCE(excluded.puuid, games.puuid),
                    queue_id = excluded.queue_id,
                    map_id = excluded.map_id,
                    game_mode = excluded.game_mode,
                    game_creation = excluded.game_creation,
                    game_duration = excluded.game_duration,
                    game_version = COALESCE(excluded.game_version, games.game_version),
                    patch = COALESCE(excluded.patch, games.patch),
                    updated_at = CURRENT_TIMESTAMP",
                rusqlite::params![
                    game_id,
                    puuid,
                    game.get("queueId").and_then(Value::as_i64),
                    game.get("mapId").and_then(Value::as_i64),
                    game.get("gameMode").and_then(Value::as_str),
                    game.get("gameCreation").and_then(Value::as_i64),
                    game.get("gameDuration").and_then(Value::as_i64),
                    game_version,
                    patch,
                ],
            )
            .map_err(|error| format!("failed storing game {game_id}: {error}"))?;

        if let Some(participants) = game.get("participants").and_then(Value::as_array) {
            for participant in participants {
                let Some(participant_id) = participant.get("participantId").and_then(Value::as_i64)
                else {
                    continue;
                };
                let stats = participant.get("stats");
                let item_ids = normalize_item_ids(
                    (0..=6)
                        .filter_map(|index| {
                            let key = format!("item{index}");
                            stats
                                .and_then(|stats| stats.get(&key))
                                .and_then(Value::as_i64)
                        })
                        .collect(),
                );
                let augment_ids = participant_augment_ids(participant);
                transaction
                    .execute(
                        "INSERT INTO game_build_facts (
                        game_id, participant_id, puuid, champion_id, win,
                        item_ids_json, augment_ids_json, kills, deaths, assists, damage, gold
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                    ON CONFLICT(game_id, participant_id) DO UPDATE SET
                        puuid = excluded.puuid,
                        champion_id = excluded.champion_id,
                        win = excluded.win,
                        item_ids_json = excluded.item_ids_json,
                        augment_ids_json = excluded.augment_ids_json,
                        kills = excluded.kills,
                        deaths = excluded.deaths,
                        assists = excluded.assists,
                        damage = excluded.damage,
                        gold = excluded.gold,
                        updated_at = CURRENT_TIMESTAMP",
                        rusqlite::params![
                            game_id,
                            participant_id,
                            puuid,
                            participant.get("championId").and_then(Value::as_i64),
                            stats
                                .and_then(|stats| stats.get("win"))
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            serde_json::to_string(&item_ids).unwrap_or_else(|_| "[]".to_string()),
                            serde_json::to_string(&augment_ids)
                                .unwrap_or_else(|_| "[]".to_string()),
                            stats
                                .and_then(|stats| stats.get("kills"))
                                .and_then(Value::as_i64)
                                .unwrap_or(0),
                            stats
                                .and_then(|stats| stats.get("deaths"))
                                .and_then(Value::as_i64)
                                .unwrap_or(0),
                            stats
                                .and_then(|stats| stats.get("assists"))
                                .and_then(Value::as_i64)
                                .unwrap_or(0),
                            stats
                                .and_then(|stats| stats.get("totalDamageDealtToChampions"))
                                .and_then(Value::as_i64),
                            stats
                                .and_then(|stats| stats.get("goldEarned"))
                                .and_then(Value::as_i64)
                                .unwrap_or(0),
                        ],
                    )
                    .map_err(|error| {
                        format!("failed storing build facts for game {game_id}: {error}")
                    })?;
            }
        }
    }
    transaction
        .commit()
        .map_err(|error| format!("failed committing games transaction: {error}"))?;
    if new_game_discovered {
        invalidate_statistics_projections(&connection)?;
    }
    Ok((games.len(), new_game_discovered))
}

fn is_aram_mayhem(game: &Value) -> bool {
    game.get("gameMode").and_then(Value::as_str) == Some("KIWI")
        || game.get("game_mode").and_then(Value::as_str) == Some("KIWI")
}

fn is_game_summary_on_patch(game: &Value, target_patch: Option<&str>) -> bool {
    let Some(target) = target_patch else {
        return true;
    };
    match game
        .get("gameVersion")
        .and_then(Value::as_str)
        .and_then(crate::services::patch_manager::normalize_patch)
    {
        Some(game_patch) => game_patch == target,
        // If summary does not include gameVersion, allow detail fetch to verify it
        None => true,
    }
}

fn is_game_detail_on_patch(detail: &Value, target_patch: Option<&str>) -> bool {
    let Some(target) = target_patch else {
        return true;
    };
    detail
        .get("gameVersion")
        .and_then(Value::as_str)
        .and_then(crate::services::patch_manager::normalize_patch)
        .as_deref()
        == Some(target)
}

fn global_game_count(path: &PathBuf) -> Result<usize, String> {
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global ARAM database for count: {error}"))?;
    connection
        .query_row("SELECT COUNT(*) FROM aram_games", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|count| count as usize)
        .map_err(|error| format!("failed counting global ARAM games: {error}"))
}

fn queue_global_account(path: &PathBuf, puuid: &str) -> Result<(), String> {
    let connection = open_connection(path).map_err(|error| {
        format!("failed opening global ARAM database for account queue: {error}")
    })?;
    connection
        .execute(
            "INSERT OR IGNORE INTO discovered_accounts (puuid) VALUES (?1)",
            rusqlite::params![puuid],
        )
        .map(|_| ())
        .map_err(|error| format!("failed queueing global account: {error}"))
}

fn mark_global_account(path: &PathBuf, puuid: &str, status: &str) -> Result<(), String> {
    let connection = open_connection(path).map_err(|error| {
        format!("failed opening global ARAM database for account state: {error}")
    })?;
    connection
        .execute(
            "UPDATE discovered_accounts SET status = ?2, updated_at = CURRENT_TIMESTAMP WHERE puuid = ?1",
            rusqlite::params![puuid, status],
        )
        .map(|_| ())
        .map_err(|error| format!("failed updating global account state: {error}"))
}

fn reset_fetching_global_accounts(path: &PathBuf) -> Result<usize, String> {
    let connection = open_connection(path).map_err(|error| {
        format!("failed opening global ARAM database for account reset: {error}")
    })?;
    connection
        .execute(
            "UPDATE discovered_accounts SET status = 'queued', updated_at = CURRENT_TIMESTAMP WHERE status = 'fetching'",
            [],
        )
        .map_err(|error| format!("failed resetting in-flight global accounts: {error}"))
}

fn claim_next_queued_account(path: &PathBuf) -> Result<Option<String>, String> {
    let connection = open_connection(path).map_err(|error| {
        format!("failed opening global ARAM database to claim account: {error}")
    })?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("failed starting account claim transaction: {error}"))?;
    let puuid: Option<String> = transaction
        .query_row(
            "SELECT puuid FROM discovered_accounts WHERE status = 'queued' ORDER BY discovered_at ASC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("failed querying next queued account: {error}"))?;
    if let Some(ref p) = puuid {
        transaction
            .execute(
                "UPDATE discovered_accounts SET status = 'fetching', updated_at = CURRENT_TIMESTAMP WHERE puuid = ?1",
                rusqlite::params![p],
            )
            .map_err(|error| format!("failed marking account fetching: {error}"))?;
    }
    transaction
        .commit()
        .map_err(|error| format!("failed committing account claim: {error}"))?;
    Ok(puuid)
}

fn global_fetching_account_count(path: &PathBuf) -> Result<usize, String> {
    let connection = open_connection(path).map_err(|error| {
        format!("failed opening global ARAM database for fetching count: {error}")
    })?;
    connection
        .query_row(
            "SELECT count(*) FROM discovered_accounts WHERE status = 'fetching'",
            [],
            |row| row.get(0),
        )
        .map(|count: i64| count as usize)
        .map_err(|error| format!("failed counting fetching global accounts: {error}"))
}

fn seed_accounts_from_existing_participants(path: &PathBuf) -> Result<usize, String> {
    let connection = open_connection(path).map_err(|error| {
        format!("failed opening global ARAM database for account seeding: {error}")
    })?;
    connection
        .execute(
            "INSERT OR IGNORE INTO discovered_accounts (puuid)
             SELECT DISTINCT puuid FROM aram_participants
             WHERE puuid IS NOT NULL AND puuid != ''",
            [],
        )
        .map_err(|error| format!("failed seeding accounts from participants: {error}"))
}

fn uncollected_global_games(path: &PathBuf, game_ids: &[i64]) -> Result<Vec<i64>, String> {
    if game_ids.is_empty() {
        return Ok(Vec::new());
    }
    let connection = open_connection(path).map_err(|error| {
        format!("failed opening global ARAM database for uncollected games: {error}")
    })?;
    let placeholders = game_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!("SELECT game_id FROM aram_games WHERE game_id IN ({placeholders})");
    let mut statement = connection
        .prepare(&query)
        .map_err(|error| format!("failed preparing uncollected games query: {error}"))?;
    let params = rusqlite::params_from_iter(game_ids.iter());
    let rows = statement
        .query_map(params, |row| row.get::<_, i64>(0))
        .map_err(|error| format!("failed querying stored games: {error}"))?;
    let mut stored = HashSet::new();
    for row in rows {
        if let Ok(id) = row {
            stored.insert(id);
        }
    }
    Ok(game_ids
        .iter()
        .copied()
        .filter(|id| !stored.contains(id))
        .collect())
}

fn global_queued_account_count(path: &PathBuf) -> Result<usize, String> {
    let connection = open_connection(path).map_err(|error| {
        format!("failed opening global ARAM database for queued count: {error}")
    })?;
    connection
        .query_row(
            "SELECT count(*) FROM discovered_accounts WHERE status = 'queued'",
            [],
            |row| row.get(0),
        )
        .map(|count: i64| count as usize)
        .map_err(|error| format!("failed counting queued global accounts: {error}"))
}

fn global_patch_breakdown(path: &PathBuf) -> Result<Vec<(String, usize)>, String> {
    let connection = open_connection(path).map_err(|error| {
        format!("failed opening global ARAM database for patch breakdown: {error}")
    })?;
    let mut statement = connection
        .prepare(
            "SELECT COALESCE(NULLIF(patch, ''), 'unknown') AS p, COUNT(*)
             FROM aram_games
             GROUP BY p
             ORDER BY COUNT(*) DESC",
        )
        .map_err(|error| format!("failed preparing global patch breakdown query: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })
        .map_err(|error| format!("failed reading global patch breakdown: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed collecting global patch breakdown: {error}"))
}

fn store_global_detail(
    path: &PathBuf,
    source_puuid: &str,
    detail: &Value,
    target_patch: Option<&str>,
) -> Result<bool, String> {
    let game_id = detail
        .get("gameId")
        .and_then(Value::as_i64)
        .ok_or_else(|| "global ARAM detail did not contain gameId".to_string())?;
    let game_version = detail.get("gameVersion").and_then(Value::as_str);
    let patch = game_version.and_then(crate::services::patch_manager::normalize_patch);
    if let Some(target) = target_patch {
        if patch.as_deref() != Some(target) {
            return Ok(false);
        }
    }
    let mut connection = open_connection(path)
        .map_err(|error| format!("failed opening global ARAM database for detail: {error}"))?;
    let game_exists = connection
        .query_row(
            "SELECT 1 FROM aram_games WHERE game_id = ?1 LIMIT 1",
            rusqlite::params![game_id],
            |_| Ok(()),
        )
        .is_ok();
    if game_exists {
        return Ok(false);
    }
    let detail_json = serde_json::to_string(detail)
        .map_err(|error| format!("failed encoding global ARAM detail {game_id}: {error}"))?;

    let tx = connection
        .transaction()
        .map_err(|error| format!("failed starting transaction for game {game_id}: {error}"))?;

    tx.execute(
        "INSERT OR IGNORE INTO aram_games (
        game_id, source_puuid, queue_id, map_id, game_mode,
        game_creation, game_duration, game_version, patch, detail_json
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            game_id,
            source_puuid,
            detail.get("queueId").and_then(Value::as_i64),
            detail.get("mapId").and_then(Value::as_i64),
            detail
                .get("gameMode")
                .and_then(Value::as_str)
                .unwrap_or("KIWI"),
            detail.get("gameCreation").and_then(Value::as_i64),
            detail.get("gameDuration").and_then(Value::as_i64),
            game_version,
            patch,
            detail_json,
        ],
    )
    .map_err(|error| format!("failed storing global ARAM game {game_id}: {error}"))?;

    let identities = detail
        .get("participantIdentities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let participants = detail
        .get("participants")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for participant in participants {
        let participant_id = participant
            .get("participantId")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let puuid = identities
            .iter()
            .find(|identity| {
                identity.get("participantId").and_then(Value::as_i64) == Some(participant_id)
            })
            .and_then(|identity| identity.get("player"))
            .and_then(|player| player.get("puuid"))
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(puuid) = puuid.as_deref() {
            let _ = tx.execute(
                "INSERT OR IGNORE INTO discovered_accounts (puuid) VALUES (?1)",
                rusqlite::params![puuid],
            );
        }
        let stats = participant.get("stats");
        let item_ids = normalize_item_ids(
            (0..=6)
                .filter_map(|index| {
                    let key = format!("item{index}");
                    stats
                        .and_then(|value| value.get(key.as_str()))
                        .and_then(Value::as_i64)
                })
                .collect(),
        );
        let augment_ids = participant_augment_ids(&participant);
        tx.execute(
            "INSERT OR REPLACE INTO aram_participants (
            game_id, participant_id, puuid, champion_id, win,
            item_ids_json, augment_ids_json, kills, deaths, assists, damage, gold
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                game_id,
                participant_id,
                puuid,
                participant.get("championId").and_then(Value::as_i64),
                stats
                    .and_then(|value| value.get("win"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                serde_json::to_string(&item_ids).unwrap_or_else(|_| "[]".to_string()),
                serde_json::to_string(&augment_ids).unwrap_or_else(|_| "[]".to_string()),
                stats
                    .and_then(|value| value.get("kills"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
                stats
                    .and_then(|value| value.get("deaths"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
                stats
                    .and_then(|value| value.get("assists"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
                stats
                    .and_then(|value| value.get("totalDamageDealtToChampions"))
                    .and_then(Value::as_i64),
                stats
                    .and_then(|value| value.get("goldEarned"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
            ],
        )
        .map_err(|error| {
            format!("failed storing global ARAM participant {game_id}:{participant_id}: {error}")
        })?;
    }

    tx.commit()
        .map_err(|error| format!("failed committing global ARAM game {game_id}: {error}"))?;

    // Throttled projection invalidation: only invalidate projections at most once every 60 seconds
    // during active batch crawling to prevent UI lockup and disk I/O thrashing.
    static LAST_PROJECTION_INVALIDATION: std::sync::Mutex<Option<std::time::Instant>> =
        std::sync::Mutex::new(None);
    let should_invalidate = {
        let mut last = LAST_PROJECTION_INVALIDATION.lock().unwrap();
        match *last {
            Some(instant) if instant.elapsed() < Duration::from_secs(60) => false,
            _ => {
                *last = Some(std::time::Instant::now());
                true
            }
        }
    };
    if should_invalidate {
        let _ = invalidate_statistics_projections(&connection);
    }
    Ok(true)
}

/// Manages request pacing for crawler and collector workers.
///
/// Enforces a maximum rate (e.g. 100 requests/minute = 600ms interval)
/// ensuring requests are evenly spaced out while not being bottlenecked when a single request takes too long.
#[derive(Debug)]
pub struct WorkerRatePacer {
    interval: Duration,
    last_request_start: Option<std::time::Instant>,
}

impl WorkerRatePacer {
    pub fn new(max_per_minute: u32) -> Self {
        let millis = if max_per_minute > 0 {
            (60_000 + max_per_minute as u64 - 1) / max_per_minute as u64
        } else {
            600
        };
        Self {
            interval: Duration::from_millis(millis),
            last_request_start: None,
        }
    }

    /// Delays execution until at least `interval` has elapsed since the start of the previous request.
    pub async fn pace_next(&mut self) {
        if let Some(prev_start) = self.last_request_start {
            let elapsed = prev_start.elapsed();
            if elapsed < self.interval {
                tokio::time::sleep(self.interval - elapsed).await;
            }
        }
        self.last_request_start = Some(std::time::Instant::now());
    }
}

/// Dynamic, cross-worker adaptive rate limiter implementing AIMD with ceiling discovery and locking.
/// Automatically discovers the sustainable LCU requests-per-minute (RPM) ceiling per route, locks onto it,
/// and deduplicates concurrent in-flight worker failures so a single choke event only backs off once.
#[derive(Debug)]
pub struct AdaptiveRateLimiter {
    current_rpm: u32,
    min_rpm: u32,
    max_rpm: u32,
    min_spacing_ms: u64,
    max_burst_per_sec: u32,
    burst_window: std::collections::VecDeque<std::time::Instant>,
    learned_ceiling_rpm: Option<u32>,
    success_streak: u32,
    step_up: u32,
    success_threshold: u32,
    ceiling_probe_threshold: u32,
    last_request_start: Option<std::time::Instant>,
    last_backoff_at: Option<std::time::Instant>,
}

impl AdaptiveRateLimiter {
    pub fn new(initial_rpm: u32, min_rpm: u32, max_rpm: u32) -> Self {
        Self::with_ceiling(initial_rpm, min_rpm, max_rpm, None)
    }

    pub fn with_ceiling(
        initial_rpm: u32,
        min_rpm: u32,
        max_rpm: u32,
        ceiling_rpm: Option<u32>,
    ) -> Self {
        Self::with_burst(initial_rpm, min_rpm, max_rpm, ceiling_rpm, 60, 12, 2, 80)
    }

    pub fn with_burst(
        initial_rpm: u32,
        min_rpm: u32,
        max_rpm: u32,
        ceiling_rpm: Option<u32>,
        min_spacing_ms: u64,
        max_burst_per_sec: u32,
        step_up: u32,
        ceiling_probe_threshold: u32,
    ) -> Self {
        let clamped_ceiling = ceiling_rpm.map(|c| c.clamp(min_rpm, max_rpm));
        let effective_max = clamped_ceiling.unwrap_or(max_rpm);
        Self {
            current_rpm: initial_rpm.clamp(min_rpm, effective_max),
            min_rpm,
            max_rpm,
            min_spacing_ms,
            max_burst_per_sec,
            burst_window: std::collections::VecDeque::new(),
            learned_ceiling_rpm: clamped_ceiling,
            success_streak: 0,
            step_up,
            success_threshold: 8,
            ceiling_probe_threshold,
            last_request_start: None,
            last_backoff_at: None,
        }
    }

    pub fn current_rpm(&self) -> u32 {
        self.current_rpm
    }

    pub fn ceiling_rpm(&self) -> Option<u32> {
        self.learned_ceiling_rpm
    }

    /// Calculates required inter-request delay given `current_rpm`.
    fn interval(&self) -> Duration {
        let rpm = self.current_rpm.max(1) as u64;
        let millis = (60_000 + rpm - 1) / rpm;
        Duration::from_millis(millis)
    }

    /// Atomically gates the next request across all workers and returns `(current_rpm, request_started_at)`.
    pub async fn pace_next_tagged(&mut self) -> (u32, std::time::Instant) {
        loop {
            let now = std::time::Instant::now();
            let mut wait = Duration::ZERO;

            if let Some(prev_start) = self.last_request_start {
                let spacing = Duration::from_millis(self.min_spacing_ms);
                let elapsed = now.saturating_duration_since(prev_start);
                if elapsed < spacing {
                    wait = wait.max(spacing - elapsed);
                }

                let sustained = self.interval();
                if elapsed < sustained {
                    wait = wait.max(sustained - elapsed);
                }
            }

            let one_sec_ago = now.checked_sub(Duration::from_secs(1)).unwrap_or(now);
            while let Some(&front) = self.burst_window.front() {
                if front <= one_sec_ago {
                    self.burst_window.pop_front();
                } else {
                    break;
                }
            }
            if self.burst_window.len() >= self.max_burst_per_sec as usize {
                if let Some(&oldest) = self.burst_window.front() {
                    let burst_wait = Duration::from_secs(1)
                        .saturating_sub(now.saturating_duration_since(oldest));
                    wait = wait.max(burst_wait);
                }
            }

            if wait.is_zero() {
                let started_at = std::time::Instant::now();
                self.last_request_start = Some(started_at);
                self.burst_window.push_back(started_at);
                return (self.current_rpm, started_at);
            }

            tokio::time::sleep(wait).await;
        }
    }

    /// Atomically gates the next request across all workers and returns current RPM.
    pub async fn pace_next(&mut self) -> u32 {
        self.pace_next_tagged().await.0
    }

    /// Records a successful request.
    /// - When no ceiling is known yet, increases RPM by `step_up` (+2 or +5) every `success_threshold` (8) successes.
    /// - When below a learned ceiling, steps up cautiously (+1 RPM) every `success_threshold` (8) successes up to the ceiling.
    /// - Once at the learned ceiling, holds steady and only probes +2 RPM after `ceiling_probe_threshold` (16) consecutive successes.
    pub fn record_success(&mut self) -> Option<u32> {
        self.success_streak += 1;
        match self.learned_ceiling_rpm {
            Some(ceiling) if self.current_rpm >= ceiling => {
                if self.success_streak >= self.ceiling_probe_threshold {
                    self.success_streak = 0;
                    if self.current_rpm < self.max_rpm {
                        let next_rpm = (self.current_rpm + 2).min(self.max_rpm);
                        self.current_rpm = next_rpm;
                        self.learned_ceiling_rpm = Some(next_rpm);
                        return Some(self.current_rpm);
                    }
                }
                None
            }
            Some(ceiling) => {
                if self.success_streak >= self.success_threshold {
                    self.success_streak = 0;
                    let old = self.current_rpm;
                    self.current_rpm = (self.current_rpm + 1).min(ceiling).min(self.max_rpm);
                    if self.current_rpm != old {
                        return Some(self.current_rpm);
                    }
                }
                None
            }
            None => {
                if self.success_streak >= self.success_threshold {
                    self.success_streak = 0;
                    if self.current_rpm < self.max_rpm {
                        let old = self.current_rpm;
                        self.current_rpm = (self.current_rpm + self.step_up).min(self.max_rpm);
                        if self.current_rpm != old {
                            return Some(self.current_rpm);
                        }
                    }
                }
                None
            }
        }
    }

    /// Records a rate-limiting event (HTTP 429) or transient timeout/error for a request started at `request_started_at`.
    /// Ignores stale errors from sibling worker requests that were already in flight before the most recent backoff.
    pub fn record_backoff_for_request(
        &mut self,
        is_429: bool,
        request_started_at: std::time::Instant,
    ) -> Option<u32> {
        if self
            .last_backoff_at
            .is_some_and(|last| request_started_at <= last)
        {
            return None;
        }
        self.last_backoff_at = Some(std::time::Instant::now());
        Some(self.apply_backoff(is_429))
    }

    /// Records a rate-limiting event (HTTP 429) or transient timeout/error unconditionally.
    pub fn record_backoff(&mut self, is_429: bool) -> u32 {
        self.last_backoff_at = Some(std::time::Instant::now());
        self.apply_backoff(is_429)
    }

    fn apply_backoff(&mut self, is_429: bool) -> u32 {
        self.success_streak = 0;
        let safe_ceiling_factor = if is_429 { 0.80 } else { 0.85 };
        let discovered_safe = ((self.current_rpm as f32 * safe_ceiling_factor) as u32)
            .min(self.current_rpm.saturating_sub(1))
            .clamp(self.min_rpm, self.max_rpm);
        self.learned_ceiling_rpm = Some(match self.learned_ceiling_rpm {
            Some(prev) => ((prev + discovered_safe) / 2).clamp(self.min_rpm, self.max_rpm),
            None => discovered_safe,
        });

        let factor = if is_429 { 0.50 } else { 0.75 };
        let mut new_rpm =
            ((self.current_rpm as f32 * factor) as u32).clamp(self.min_rpm, self.max_rpm);
        if let Some(ceiling) = self.learned_ceiling_rpm {
            new_rpm = new_rpm.min(ceiling);
        }
        self.current_rpm = new_rpm;
        new_rpm
    }
}

fn load_route_rate_limit(
    path: &PathBuf,
    route_key: &str,
    default_rpm: u32,
    min_rpm: u32,
    max_rpm: u32,
) -> (u32, Option<u32>) {
    let Ok(conn) = open_connection(path) else {
        return (default_rpm.clamp(min_rpm, max_rpm), None);
    };
    let rpm_key = format!("rate_limit_{route_key}_rpm");
    let ceiling_key = format!("rate_limit_{route_key}_ceiling_rpm");

    let saved_rpm = conn
        .query_row(
            "SELECT value FROM config WHERE key = ?1",
            rusqlite::params![rpm_key],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|&v| v >= default_rpm)
        .map(|v| v.clamp(min_rpm, max_rpm));

    let saved_ceiling = conn
        .query_row(
            "SELECT value FROM config WHERE key = ?1",
            rusqlite::params![ceiling_key],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|&v| v > default_rpm)
        .map(|v| v.clamp(min_rpm, max_rpm));

    let initial_rpm = saved_rpm
        .unwrap_or(default_rpm)
        .clamp(min_rpm, saved_ceiling.unwrap_or(max_rpm));
    (initial_rpm, saved_ceiling)
}

fn save_route_rate_limit(
    path: &PathBuf,
    route_key: &str,
    current_rpm: u32,
    ceiling_rpm: Option<u32>,
) {
    let Ok(conn) = open_connection(path) else {
        return;
    };
    let rpm_key = format!("rate_limit_{route_key}_rpm");
    let _ = conn.execute(
        "INSERT INTO config(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![rpm_key, current_rpm.to_string()],
    );
    if let Some(ceiling) = ceiling_rpm {
        let ceiling_key = format!("rate_limit_{route_key}_ceiling_rpm");
        let _ = conn.execute(
            "INSERT INTO config(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![ceiling_key, ceiling.to_string()],
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CooldownOutcome {
    NoCooldown,
    Triggered {
        streak: u32,
        cooldown_secs: u64,
        next_backoff_secs: u64,
    },
    JoinedExisting {
        remaining_secs: u64,
    },
}

struct CrawlerCircuitBreaker {
    consecutive_errors: u32,
    cooldown_until: Option<std::time::Instant>,
    current_backoff_secs: u64,
    last_error_at: Option<std::time::Instant>,
}

impl CrawlerCircuitBreaker {
    fn new() -> Self {
        Self {
            consecutive_errors: 0,
            cooldown_until: None,
            current_backoff_secs: 2, // 2 seconds initial backoff instead of 60s
            last_error_at: None,
        }
    }

    fn is_in_cooldown(&self) -> Option<Duration> {
        if let Some(until) = self.cooldown_until {
            let now = std::time::Instant::now();
            if until > now {
                return Some(until - now);
            }
        }
        None
    }

    /// Records a failure for a request that started at `request_started_at`.
    /// - If a cooldown is ALREADY active, does NOT increment `consecutive_errors` or double `current_backoff_secs`;
    ///   instead returns `JoinedExisting` so the sibling worker simply waits out the existing cooldown.
    /// - If the request started before the most recent error (`request_started_at <= last_error_at`),
    ///   treats it as part of the same concurrent wave without escalating the error streak.
    fn record_failure_for_request(
        &mut self,
        is_429: bool,
        retry_after: Option<Duration>,
        request_started_at: std::time::Instant,
    ) -> CooldownOutcome {
        if let Some(remaining) = self.is_in_cooldown() {
            return CooldownOutcome::JoinedExisting {
                remaining_secs: remaining.as_secs().max(1),
            };
        }

        let is_stale_in_flight = self
            .last_error_at
            .is_some_and(|last| request_started_at <= last);
        if is_stale_in_flight && !is_429 {
            return CooldownOutcome::NoCooldown;
        }

        self.last_error_at = Some(std::time::Instant::now());
        if is_429 {
            let cooldown = self.record_rate_limit(retry_after);
            CooldownOutcome::Triggered {
                streak: self.consecutive_errors,
                cooldown_secs: cooldown,
                next_backoff_secs: self.current_backoff_secs,
            }
        } else if let Some((streak, cooldown_secs, next_backoff_secs)) = self.record_error() {
            CooldownOutcome::Triggered {
                streak,
                cooldown_secs,
                next_backoff_secs,
            }
        } else {
            CooldownOutcome::NoCooldown
        }
    }

    fn record_rate_limit(&mut self, retry_after: Option<Duration>) -> u64 {
        if let Some(remaining) = self.is_in_cooldown() {
            return remaining.as_secs().max(1);
        }
        self.consecutive_errors += 1;
        let cooldown = match retry_after {
            Some(dur) => dur.as_secs().max(1),
            None => self.current_backoff_secs.max(2),
        };
        self.cooldown_until = Some(std::time::Instant::now() + Duration::from_secs(cooldown));
        self.current_backoff_secs = (cooldown * 2).clamp(2, 60);
        cooldown
    }

    fn record_error(&mut self) -> Option<(u32, u64, u64)> {
        if let Some(remaining) = self.is_in_cooldown() {
            let rem = remaining.as_secs().max(1);
            return Some((self.consecutive_errors, rem, self.current_backoff_secs));
        }
        self.consecutive_errors += 1;
        if self.consecutive_errors >= 4 {
            let cooldown = self.current_backoff_secs;
            self.cooldown_until = Some(std::time::Instant::now() + Duration::from_secs(cooldown));
            let next_backoff = (self.current_backoff_secs * 2).min(60);
            self.current_backoff_secs = next_backoff;
            Some((self.consecutive_errors, cooldown, next_backoff))
        } else {
            None
        }
    }

    fn record_success(&mut self) {
        if self.consecutive_errors > 0 {
            self.consecutive_errors = 0;
            self.current_backoff_secs = 2;
            self.cooldown_until = None;
        }
    }
}

async fn run_global_aram_worker(
    worker_id: usize,
    generation: u64,
    client: crate::services::lcu_client::LcuClient,
    path: PathBuf,
    runtime: Arc<RwLock<RuntimeState>>,
    in_flight_games: Arc<tokio::sync::Mutex<HashSet<i64>>>,
    circuit_breaker: Arc<tokio::sync::Mutex<CrawlerCircuitBreaker>>,
    matches_limiter: Arc<tokio::sync::Mutex<AdaptiveRateLimiter>>,
    games_limiter: Arc<tokio::sync::Mutex<AdaptiveRateLimiter>>,
    matches_gate: Arc<tokio::sync::Mutex<()>>,
) {
    logger::info(
        "agent.global-aram",
        &format!("Global ARAM worker #{worker_id} started (gen {generation})"),
    );
    let mut empty_streak = 0;

    loop {
        if CRAWLER_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
            logger::info(
                "agent.global-aram",
                &format!("Worker #{worker_id} stopping due to database reset"),
            );
            break;
        }
        if runtime
            .read()
            .await
            .collector_state
            .starts_with("game_in_progress")
        {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        // If the circuit breaker is active, pause until cooldown expires
        let wait_dur = {
            let cb = circuit_breaker.lock().await;
            cb.is_in_cooldown()
        };
        if let Some(dur) = wait_dur {
            tokio::time::sleep(dur).await;
        }

        let live_limit = match global_game_limit(&path) {
            Ok(limit) => limit,
            Err(_) => break,
        };
        let live_count = match global_game_count(&path) {
            Ok(count) => count,
            Err(_) => break,
        };
        if live_count >= live_limit {
            break;
        }

        let claim_path = path.clone();
        let account_puuid =
            match tokio::task::spawn_blocking(move || claim_next_queued_account(&claim_path)).await
            {
                Ok(Ok(Some(puuid))) => {
                    empty_streak = 0;
                    puuid
                }
                Ok(Ok(None)) => {
                    empty_streak += 1;
                    if empty_streak <= 5 {
                        let seed_path = path.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            seed_accounts_from_existing_participants(&seed_path)
                        })
                        .await;
                        tokio::time::sleep(Duration::from_millis(1000)).await;
                        continue;
                    } else {
                        let check_path = path.clone();
                        let fetching_count = tokio::task::spawn_blocking(move || {
                            global_fetching_account_count(&check_path)
                        })
                        .await
                        .unwrap_or(Ok(0))
                        .unwrap_or(0);
                        if fetching_count > 0 {
                            tokio::time::sleep(Duration::from_millis(1500)).await;
                            continue;
                        }
                        logger::info(
                            "agent.global-aram",
                            &format!(
                            "Worker #{worker_id}: queue exhausted and no active fetches; stopping"
                        ),
                        );
                        break;
                    }
                }
                Ok(Err(err)) => {
                    logger::error(
                        "agent.global-aram",
                        &format!("Worker #{worker_id} claim error: {err}"),
                    );
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    continue;
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    continue;
                }
            };

        let (history_res, req_started_at) = {
            let _matches_guard = matches_gate.lock().await;

            // Shared adaptive rate limiter for the `/matches` route across all workers
            let (_, req_started_at) = matches_limiter.lock().await.pace_next_tagged().await;

            // Re-check circuit breaker after waiting on the pacer in case a sibling worker just triggered cooldown
            if let Some(dur) = circuit_breaker.lock().await.is_in_cooldown() {
                let mark_path = path.clone();
                let mark_puuid = account_puuid.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    mark_global_account(&mark_path, &mark_puuid, "queued")
                })
                .await;
                tokio::time::sleep(dur).await;
                continue;
            }

            let route = format!(
                "/lol-match-history/v1/products/lol/{account_puuid}/matches?begIndex=0&endIndex=19"
            );
            let resp = client.get_json_response::<Value>(&route).await;
            (resp, req_started_at)
        };

        let history = match history_res {
            Ok(history) => {
                circuit_breaker.lock().await.record_success();
                let maybe_update = {
                    let mut rl = matches_limiter.lock().await;
                    rl.record_success().map(|rpm| (rpm, rl.ceiling_rpm()))
                };
                if let Some((new_rpm, ceiling)) = maybe_update {
                    let save_path = path.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        save_route_rate_limit(&save_path, "matches", new_rpm, ceiling);
                    })
                    .await;
                    logger::info(
                        "agent.global-aram",
                        &format!(
                            "Crawler `/matches` rate increased: now operating at {new_rpm} req/min (ceiling: {:?})",
                            ceiling
                        ),
                    );
                }
                history
            }
            Err(error) => {
                logger::warn(
                    "agent.global-aram",
                    &format!("Worker #{worker_id}: Match history unavailable for account {account_puuid}: {error}"),
                );
                let mark_path = path.clone();
                let mark_puuid = account_puuid.clone();

                let is_rate_limited = error.status == Some(429);
                let retry_after = error.retry_after;
                let maybe_backoff = {
                    let mut rl = matches_limiter.lock().await;
                    rl.record_backoff_for_request(is_rate_limited, req_started_at)
                        .map(|rpm| (rpm, rl.ceiling_rpm()))
                };
                if let Some((new_rpm, ceiling)) = maybe_backoff {
                    let save_path = path.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        save_route_rate_limit(&save_path, "matches", new_rpm, ceiling);
                    })
                    .await;
                    logger::info(
                        "agent.global-aram",
                        &format!(
                            "Crawler `/matches` rate adjusted down: now operating at {new_rpm} req/min (learned ceiling: {:?})",
                            ceiling
                        ),
                    );
                }

                let outcome = circuit_breaker.lock().await.record_failure_for_request(
                    is_rate_limited,
                    retry_after,
                    req_started_at,
                );

                match outcome {
                    CooldownOutcome::Triggered {
                        streak,
                        cooldown_secs,
                        next_backoff_secs,
                    } => {
                        let mins = (cooldown_secs + 59) / 60;
                        let next_mins = (next_backoff_secs + 59) / 60;
                        let reason = if is_rate_limited {
                            "HTTP 429 Rate Limit"
                        } else {
                            "unavailable consecutively"
                        };
                        logger::warn(
                            "agent.global-aram",
                            &format!(
                                "Match history {reason} ({streak} errors). Backing off all workers for {cooldown_secs}s before retrying (next backoff: {next_mins}m)..."
                            ),
                        );
                        {
                            let mut state = runtime.write().await;
                            state.collector_state = format!("global-aram:paused (backoff {mins}m)");
                        }
                        // Re-queue this account so it is not permanently lost due to the LCU outage
                        let _ = tokio::task::spawn_blocking(move || {
                            mark_global_account(&mark_path, &mark_puuid, "queued")
                        })
                        .await;
                        tokio::time::sleep(Duration::from_secs(cooldown_secs)).await;
                    }
                    CooldownOutcome::JoinedExisting { remaining_secs } => {
                        let _ = tokio::task::spawn_blocking(move || {
                            mark_global_account(&mark_path, &mark_puuid, "queued")
                        })
                        .await;
                        tokio::time::sleep(Duration::from_secs(remaining_secs)).await;
                    }
                    CooldownOutcome::NoCooldown => {
                        let _ = tokio::task::spawn_blocking(move || {
                            mark_global_account(&mark_path, &mark_puuid, "failed")
                        })
                        .await;
                    }
                }
                continue;
            }
        };

        let active_patch = runtime
            .read()
            .await
            .current_patch
            .clone()
            .or_else(crate::services::patch_manager::get_cached_patch);
        let summaries = history
            .get("games")
            .and_then(|value| value.get("games"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let all_aram_summaries: Vec<Value> = summaries.into_iter().filter(is_aram_mayhem).collect();
        let aram_summaries: Vec<Value> = all_aram_summaries
            .iter()
            .filter(|summary| is_game_summary_on_patch(summary, active_patch.as_deref()))
            .cloned()
            .collect();

        // If the seed account has no games on the current patch yet and no other accounts are queued,
        // seed discovered_accounts from their most recent ARAM Mayhem game's participants without storing the old game.
        if aram_summaries.is_empty() && !all_aram_summaries.is_empty() {
            if global_game_count(&path).unwrap_or(0) == 0
                && global_queued_account_count(&path).unwrap_or(0) == 0
            {
                if let Some(seed_game_id) =
                    all_aram_summaries[0].get("gameId").and_then(Value::as_i64)
                {
                    games_limiter.lock().await.pace_next().await;
                    if let Ok(detail) = client
                        .get_json::<Value>(&format!("/lol-match-history/v1/games/{seed_game_id}"))
                        .await
                    {
                        if let Some(identities) = detail
                            .get("participantIdentities")
                            .and_then(Value::as_array)
                        {
                            let seed_path = path.clone();
                            let puuids: Vec<String> = identities
                                .iter()
                                .filter_map(|id| id.get("player"))
                                .filter_map(|p| p.get("puuid"))
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect();
                            let _ = tokio::task::spawn_blocking(move || {
                                for p in puuids {
                                    let _ = queue_global_account(&seed_path, &p);
                                }
                            })
                            .await;
                        }
                    }
                }
            }
        }

        let aram_game_ids: Vec<i64> = aram_summaries
            .iter()
            .filter_map(|summary| summary.get("gameId").and_then(Value::as_i64))
            .collect();

        let check_path = path.clone();
        let ids_to_check = aram_game_ids.clone();
        let needed_game_ids = tokio::task::spawn_blocking(move || {
            uncollected_global_games(&check_path, &ids_to_check)
        })
        .await
        .unwrap_or(Ok(aram_game_ids.clone()))
        .unwrap_or(aram_game_ids);

        // Claim uncollected games in in_flight_games so other workers don't search the same place
        let mut games_to_fetch = Vec::new();
        {
            let mut inflight = in_flight_games.lock().await;
            for id in needed_game_ids {
                if inflight.insert(id) {
                    games_to_fetch.push(id);
                }
            }
        }

        for &game_id in &games_to_fetch {
            if CRAWLER_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
                break;
            }
            // Check if the circuit breaker cooldown is active before starting each game request
            let wait_dur = {
                let cb = circuit_breaker.lock().await;
                cb.is_in_cooldown()
            };
            if let Some(dur) = wait_dur {
                tokio::time::sleep(dur).await;
                break;
            }

            if let Ok(count) = global_game_count(&path) {
                if let Ok(limit) = global_game_limit(&path) {
                    if count >= limit {
                        break;
                    }
                }
            }

            // Shared adaptive rate limiter for the `/games/{id}` route across all workers
            let (_, game_req_started_at) = games_limiter.lock().await.pace_next_tagged().await;

            if let Some(dur) = circuit_breaker.lock().await.is_in_cooldown() {
                tokio::time::sleep(dur).await;
                break;
            }

            let result = client
                .get_json_response::<Value>(&format!("/lol-match-history/v1/games/{game_id}"))
                .await;

            match result {
                Ok(detail)
                    if is_aram_mayhem(&detail)
                        && is_game_detail_on_patch(&detail, active_patch.as_deref()) =>
                {
                    if CRAWLER_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
                        break;
                    }
                    circuit_breaker.lock().await.record_success();
                    let maybe_update = {
                        let mut rl = games_limiter.lock().await;
                        rl.record_success().map(|rpm| (rpm, rl.ceiling_rpm()))
                    };
                    if let Some((new_rpm, ceiling)) = maybe_update {
                        let save_path = path.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            save_route_rate_limit(&save_path, "games", new_rpm, ceiling);
                        })
                        .await;
                        logger::info(
                            "agent.global-aram",
                            &format!(
                                "Crawler `/games` rate increased: now operating at {new_rpm} req/min (ceiling: {:?})",
                                ceiling
                            ),
                        );
                    }
                    let write_path = path.clone();
                    let write_source = account_puuid.clone();
                    let write_patch = active_patch.clone();
                    let store_result = tokio::task::spawn_blocking(move || {
                        store_global_detail(
                            &write_path,
                            &write_source,
                            &detail,
                            write_patch.as_deref(),
                        )
                    })
                    .await;
                    if let Ok(Ok(true)) = store_result {
                        if let (Ok(count), Ok(live_limit)) =
                            (global_game_count(&path), global_game_limit(&path))
                        {
                            let mut state = runtime.write().await;
                            state.update_crawler_progress(count, live_limit);
                            logger::info(
                                "agent.global-aram",
                                &format!("Worker #{worker_id} stored game {game_id}; dataset is {count}/{live_limit}"),
                            );
                            if count >= live_limit {
                                break;
                            }
                        }
                    }
                }
                Ok(detail) if is_aram_mayhem(&detail) => {
                    // Detail is from an older patch; since match history is newest-first,
                    // all subsequent matches for this account are also from older patches.
                    circuit_breaker.lock().await.record_success();
                    let maybe_update = {
                        let mut rl = games_limiter.lock().await;
                        rl.record_success().map(|rpm| (rpm, rl.ceiling_rpm()))
                    };
                    if let Some((new_rpm, ceiling)) = maybe_update {
                        let save_path = path.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            save_route_rate_limit(&save_path, "games", new_rpm, ceiling);
                        })
                        .await;
                        logger::info(
                            "agent.global-aram",
                            &format!(
                                "Crawler `/games` rate increased: now operating at {new_rpm} req/min (ceiling: {:?})",
                                ceiling
                            ),
                        );
                    }
                    in_flight_games.lock().await.remove(&game_id);
                    break;
                }
                Ok(_) => {
                    circuit_breaker.lock().await.record_success();
                    let maybe_update = {
                        let mut rl = games_limiter.lock().await;
                        rl.record_success().map(|rpm| (rpm, rl.ceiling_rpm()))
                    };
                    if let Some((new_rpm, ceiling)) = maybe_update {
                        let save_path = path.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            save_route_rate_limit(&save_path, "games", new_rpm, ceiling);
                        })
                        .await;
                        logger::info(
                            "agent.global-aram",
                            &format!(
                                "Crawler `/games` rate increased: now operating at {new_rpm} req/min (ceiling: {:?})",
                                ceiling
                            ),
                        );
                    }
                }
                Err(error) => {
                    if error.status == Some(404) {
                        logger::info(
                            "agent.global-aram",
                            &format!("Worker #{worker_id}: Game {game_id} returned 404 (expired match); skipping"),
                        );
                        in_flight_games.lock().await.remove(&game_id);
                        continue;
                    }

                    logger::warn(
                        "agent.global-aram",
                        &format!("Worker #{worker_id} failed fetching game {game_id}: {error}"),
                    );

                    let is_rate_limited = error.status == Some(429);
                    let retry_after = error.retry_after;
                    let is_transient = error.status.is_none()
                        || error.status.map(|s| s >= 500).unwrap_or(false)
                        || error.message.contains("timed out");
                    let maybe_backoff = {
                        let mut rl = games_limiter.lock().await;
                        rl.record_backoff_for_request(is_rate_limited, game_req_started_at)
                            .map(|rpm| (rpm, rl.ceiling_rpm()))
                    };
                    if let Some((new_rpm, ceiling)) = maybe_backoff {
                        let save_path = path.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            save_route_rate_limit(&save_path, "games", new_rpm, ceiling);
                        })
                        .await;
                        logger::info(
                            "agent.global-aram",
                            &format!(
                                "Crawler `/games` rate adjusted down: now operating at {new_rpm} req/min (learned ceiling: {:?})",
                                ceiling
                            ),
                        );
                    }

                    let outcome = if is_rate_limited || is_transient {
                        circuit_breaker.lock().await.record_failure_for_request(
                            is_rate_limited,
                            retry_after,
                            game_req_started_at,
                        )
                    } else {
                        CooldownOutcome::NoCooldown
                    };

                    match outcome {
                        CooldownOutcome::Triggered {
                            streak,
                            cooldown_secs,
                            next_backoff_secs,
                        } => {
                            let mins = (cooldown_secs + 59) / 60;
                            let next_mins = (next_backoff_secs + 59) / 60;
                            let reason = if is_rate_limited {
                                "HTTP 429 Rate Limit"
                            } else {
                                "consecutive timeouts/errors"
                            };
                            logger::warn(
                                "agent.global-aram",
                                &format!(
                                    "Game fetch {reason} ({streak} errors). Backing off all workers for {cooldown_secs}s before retrying (next backoff: {next_mins}m)..."
                                ),
                            );
                            {
                                let mut state = runtime.write().await;
                                state.collector_state =
                                    format!("global-aram:paused (backoff {mins}m)");
                            }
                            tokio::time::sleep(Duration::from_secs(cooldown_secs)).await;
                            break;
                        }
                        CooldownOutcome::JoinedExisting { remaining_secs } => {
                            tokio::time::sleep(Duration::from_secs(remaining_secs)).await;
                            break;
                        }
                        CooldownOutcome::NoCooldown => {}
                    }
                }
            }
            in_flight_games.lock().await.remove(&game_id);
        }
        {
            let mut inflight = in_flight_games.lock().await;
            for id in &games_to_fetch {
                inflight.remove(id);
            }
        }

        if CRAWLER_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
            break;
        }
        let mark_path = path.clone();
        let mark_puuid = account_puuid.clone();
        let _ = tokio::task::spawn_blocking(move || {
            mark_global_account(&mark_path, &mark_puuid, "complete")
        })
        .await;
    }

    logger::info(
        "agent.global-aram",
        &format!("Global ARAM worker #{worker_id} finished"),
    );
}

async fn collect_global_aram(
    client: &crate::services::lcu_client::LcuClient,
    seed_puuid: &str,
    generation: u64,
    runtime: &Arc<RwLock<RuntimeState>>,
) -> Result<(), String> {
    let path = global_aram_database_path();
    let initialize_path = path.clone();
    tokio::task::spawn_blocking(move || initialize_global_aram_database(&initialize_path))
        .await
        .map_err(|error| format!("global database initialization failed: {error}"))??;
    let current = global_game_count(&path)?;
    let limit = global_game_limit(&path)?;
    let worker_count = global_worker_count(&path).unwrap_or(4);
    {
        let mut state = runtime.write().await;
        state
            .operations
            .retain(|operation| operation.id != "global-database");
        state.operations.push(ProgressOperation {
            id: "global-database".to_string(),
            message: "Updating global game database".to_string(),
            current: Some(current),
            total: Some(limit),
        });
    }
    if current >= limit {
        runtime
            .write()
            .await
            .operations
            .retain(|operation| operation.id != "global-database");
        return Ok(());
    }

    let reset_path = path.clone();
    let _ = tokio::task::spawn_blocking(move || reset_fetching_global_accounts(&reset_path)).await;

    let seed_part_path = path.clone();
    let _ = tokio::task::spawn_blocking(move || {
        seed_accounts_from_existing_participants(&seed_part_path)
    })
    .await;

    let _ = queue_global_account(&path, seed_puuid);

    let rate_path = path.clone();
    let ((matches_rpm, matches_ceiling), (games_rpm, games_ceiling)) =
        tokio::task::spawn_blocking(move || {
            let matches_cfg = load_route_rate_limit(&rate_path, "matches", 10, 8, 15);
            let games_cfg = load_route_rate_limit(&rate_path, "games", 120, 30, 480);
            (matches_cfg, games_cfg)
        })
        .await
        .unwrap_or(((10, None), (120, None)));

    logger::info(
        "agent.global-aram",
        &format!(
            "Starting global ARAM collection with {worker_count} concurrent workers at {}/{} games (cached `/matches`: {} RPM [ceiling {:?}], `/games`: {} RPM [ceiling {:?}])",
            current, limit, matches_rpm, matches_ceiling, games_rpm, games_ceiling
        ),
    );

    let in_flight_games = Arc::new(tokio::sync::Mutex::new(HashSet::new()));
    let circuit_breaker = Arc::new(tokio::sync::Mutex::new(CrawlerCircuitBreaker::new()));
    let matches_gate = Arc::new(tokio::sync::Mutex::new(()));
    // Separate adaptive rate limiters per route, with burst limits:
    // matches: min spacing 5500ms (5.5s), max burst 1/sec
    // games: min spacing 60ms, max burst 12/sec
    let matches_limiter = Arc::new(tokio::sync::Mutex::new(AdaptiveRateLimiter::with_burst(
        matches_rpm,
        8,
        15,
        matches_ceiling,
        5500,
        1,
        1,
        60,
    )));
    let games_limiter = Arc::new(tokio::sync::Mutex::new(AdaptiveRateLimiter::with_burst(
        games_rpm,
        30,
        480,
        games_ceiling,
        60,
        12,
        5,
        16,
    )));
    let mut handles = Vec::new();

    for worker_id in 0..worker_count {
        let client = client.clone();
        let path = path.clone();
        let runtime = runtime.clone();
        let in_flight = in_flight_games.clone();
        let cb = circuit_breaker.clone();
        let ml = matches_limiter.clone();
        let gl = games_limiter.clone();
        let mg = matches_gate.clone();
        handles.push(tokio::spawn(async move {
            run_global_aram_worker(
                worker_id, generation, client, path, runtime, in_flight, cb, ml, gl, mg,
            )
            .await;
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    if CRAWLER_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
        return Ok(());
    }

    let final_count = global_game_count(&path).unwrap_or(current);
    logger::info(
        "agent.global-aram",
        &format!(
            "Global ARAM collection finished at {}/{} games",
            final_count, limit
        ),
    );
    runtime
        .write()
        .await
        .operations
        .retain(|operation| operation.id != "global-database");
    Ok(())
}

async fn refresh_statistics_projections(
    database: PathBuf,
    global_database: PathBuf,
    runtime: Arc<RwLock<RuntimeState>>,
) {
    {
        let mut state = runtime.write().await;
        if state.projection_running {
            return;
        }
        state.projection_running = true;
        state.operations = vec![
            ProgressOperation {
                id: "personal-statistics".to_string(),
                message: "Updating personal statistics".to_string(),
                current: Some(0),
                total: Some(4),
            },
            ProgressOperation {
                id: "global-statistics".to_string(),
                message: "Updating global statistics".to_string(),
                current: Some(0),
                total: Some(4),
            },
        ];
    }

    for (scope, source_path) in [("personal", database), ("global", global_database)] {
        for (index, kind) in ["champion", "item", "augment", "combo"].iter().enumerate() {
            if scope == "personal" && *kind == "combo" {
                continue;
            }
            let args = json!({
                "scope": scope,
                "kind": kind,
                "search": "",
                "hide_low_pickrate": false,
                "sort_key": "name",
                "descending": false,
                "limit": 10000,
                "offset": 0
            });
            let database_path = source_path.clone();
            let global_path = global_aram_database_path();
            let result = tokio::task::spawn_blocking(move || {
                build_statistics_response(&args, &database_path, &global_path)
            })
            .await;
            match result {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => logger::error(
                    "agent.statistics",
                    &format!("Projection failed for {scope}/{kind}: {error}"),
                ),
                Err(error) => logger::error(
                    "agent.statistics",
                    &format!("Projection worker failed for {scope}/{kind}: {error}"),
                ),
            }
            let mut state = runtime.write().await;
            if let Some(operation) = state
                .operations
                .iter_mut()
                .find(|operation| operation.id == format!("{scope}-statistics"))
            {
                operation.current = Some(index + 1);
                operation.message = format!("Updating {scope} {} statistics", kind);
            }
        }
    }

    let mut state = runtime.write().await;
    state.operations.clear();
    state.projection_running = false;
}

fn global_game_limit(path: &PathBuf) -> Result<usize, String> {
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global ARAM config: {error}"))?;
    connection
        .query_row(
            "SELECT value FROM config WHERE key = 'game_limit'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| format!("failed reading global ARAM game limit: {error}"))
        .and_then(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid global ARAM game limit: {error}"))
        })
}

fn set_global_game_limit(path: &PathBuf, limit: usize) -> Result<(), String> {
    if !(1..=100_000).contains(&limit) {
        return Err("global ARAM game limit must be between 1 and 100000".to_string());
    }
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global ARAM config: {error}"))?;
    connection.execute(
        "INSERT INTO config (key, value, updated_at) VALUES ('game_limit', ?1, CURRENT_TIMESTAMP)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
        rusqlite::params![limit.to_string()],
    ).map(|_| ()).map_err(|error| format!("failed storing global ARAM game limit: {error}"))
}

fn global_worker_count(_path: &PathBuf) -> Result<usize, String> {
    Ok(4)
}

fn set_global_worker_count(path: &PathBuf, _count: usize) -> Result<(), String> {
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global ARAM config: {error}"))?;
    connection
        .execute(
            "INSERT INTO config (key, value, updated_at) VALUES ('worker_count', '4', CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            [],
        )
        .map(|_| ())
        .map_err(|error| format!("failed storing global ARAM worker count: {error}"))
}

fn sync_state_value(path: &PathBuf, key: &str) -> Result<Option<String>, String> {
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening agent database for sync state: {error}"))?;
    connection
        .query_row(
            "SELECT value FROM sync_state WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("failed reading sync state {key}: {error}"))
}

fn set_sync_state_value(path: &PathBuf, key: &str, value: &str) -> Result<(), String> {
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening agent database for sync state: {error}"))?;
    connection
        .execute(
            "INSERT INTO sync_state (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            rusqlite::params![key, value],
        )
        .map_err(|error| format!("failed writing sync state {key}: {error}"))?;
    Ok(())
}

fn shared_detail_cache_path(game_id: i64) -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu-companion")
        .join("cache")
        .join("match-history")
        .join(format!("{game_id}.json"))
}

fn cache_game_detail(game_id: i64, detail: &Value) -> Result<(), String> {
    let path = shared_detail_cache_path(game_id);
    let directory = path
        .parent()
        .ok_or_else(|| "match detail cache has no parent directory".to_string())?;
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("failed creating match detail cache directory: {error}"))?;
    let bytes = serde_json::to_vec(detail)
        .map_err(|error| format!("failed encoding match detail {game_id}: {error}"))?;
    std::fs::write(path, bytes)
        .map_err(|error| format!("failed writing match detail {game_id}: {error}"))
}

async fn cache_page_details(
    client: &crate::services::lcu_client::LcuClient,
    page_games: &[Value],
    runtime: &Arc<RwLock<RuntimeState>>,
    discovered: usize,
    pacer: &mut WorkerRatePacer,
) {
    let mut fetched = 0;
    let mut skipped = 0;
    let mut failed = 0;
    for game in page_games {
        let Some(game_id) = game.get("gameId").and_then(Value::as_i64) else {
            continue;
        };
        if shared_detail_cache_path(game_id).exists() {
            skipped += 1;
            continue;
        }
        let route = format!("/lol-match-history/v1/games/{game_id}");
        {
            let mut state = runtime.write().await;
            state.collector_state = format!("backfilling-details:{discovered}");
        }
        pacer.pace_next().await;
        match client.get_json::<Value>(&route).await {
            Ok(detail) => {
                if let Err(error) = cache_game_detail(game_id, &detail) {
                    logger::error(
                        "agent.collector",
                        &format!("Failed caching detail for game {game_id}: {error}"),
                    );
                    failed += 1;
                } else {
                    fetched += 1;
                }
            }
            Err(error) => {
                failed += 1;
                logger::warn(
                    "agent.collector",
                    &format!("Detail unavailable for game {game_id}: {error}"),
                );
            }
        }
    }
    logger::info(
        "agent.collector",
        &format!(
            "Backfill detail page processed: {fetched} fetched, {skipped} already cached, {failed} failed"
        ),
    );
}

async fn backfill_match_history(
    client: &crate::services::lcu_client::LcuClient,
    path: &PathBuf,
    puuid: Option<&str>,
    runtime: &Arc<RwLock<RuntimeState>>,
) -> Result<usize, String> {
    const PAGE_SIZE: u32 = 20;
    let mut begin = 0;
    let mut discovered = 0;
    let mut pages = 0;
    let mut seen_game_ids = std::collections::HashSet::new();
    let mut pacer = WorkerRatePacer::new(100);

    logger::info(
        "agent.collector",
        "Starting exhaustive match-history backfill",
    );
    loop {
        let end = begin + PAGE_SIZE - 1;
        let route = if let Some(puuid) = puuid {
            format!(
                "/lol-match-history/v1/products/lol/{puuid}/matches?begIndex={begin}&endIndex={end}"
            )
        } else {
            format!(
                "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex={begin}&endIndex={end}"
            )
        };
        logger::debug(
            "agent.collector",
            &format!("Fetching match-history page {begin}..{end}"),
        );
        pacer.pace_next().await;
        let history = client.get_json::<Value>(&route).await?;
        let page_games = history
            .get("games")
            .and_then(|value| value.get("games"))
            .and_then(Value::as_array)
            .ok_or_else(|| {
                format!("match history page {begin}..{end} did not contain games.games")
            })?;

        if page_games.is_empty() {
            logger::info(
                "agent.collector",
                &format!("Backfill reached empty page at {begin}..{end}; stopping"),
            );
            break;
        }

        let (page_count, _) = store_recent_games(path, &history, puuid)?;
        discovered += page_count;
        cache_page_details(client, page_games, runtime, discovered, &mut pacer).await;
        let new_game_ids = page_games
            .iter()
            .filter_map(|game| game.get("gameId").and_then(Value::as_i64))
            .filter(|game_id| seen_game_ids.insert(*game_id))
            .count();
        pages += 1;
        {
            let mut state = runtime.write().await;
            state.collector_state = format!("backfilling:{discovered}");
        }
        logger::info(
            "agent.collector",
            &format!(
                "Backfill page {begin}..{end}: {page_count} games ({discovered} records, {new_game_ids} new)"
            ),
        );

        if new_game_ids == 0 {
            logger::info(
                "agent.collector",
                &format!("Backfill reached a repeated page at {begin}..{end}; stopping"),
            );
            break;
        }
        begin = end + 1;

        // Prevent a malfunctioning client from producing an unbounded scan.
        if pages >= 1000 {
            logger::warn(
                "agent.collector",
                &format!("Stopping history backfill after {pages} pages"),
            );
            break;
        }
    }

    logger::info(
        "agent.collector",
        &format!("Exhaustive match-history backfill finished: {pages} pages, {discovered} records"),
    );
    Ok(discovered)
}

pub async fn collector_loop(path: PathBuf, runtime: Arc<RwLock<RuntimeState>>) {
    let mut last_reconciled_puuid: Option<String> = None;
    {
        let (initial_patch, patch_changed) =
            crate::services::patch_manager::ensure_patch_freshness(None).await;
        if let Some(ref p) = initial_patch {
            runtime.write().await.current_patch = Some(p.clone());
        }
        if patch_changed {
            let db_path = path.clone();
            let global_path = global_aram_database_path();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(conn) = open_connection(&db_path) {
                    let _ = invalidate_statistics_projections(&conn);
                }
                if let Ok(conn) = open_connection(&global_path) {
                    let _ = invalidate_statistics_projections(&conn);
                }
            })
            .await;
        }
    }
    loop {
        let credentials = crate::services::lcu_client::LcuClient::find_credentials();
        let Some(credentials) = credentials else {
            let mut state = runtime.write().await;
            state.collector_state = "sleeping".to_string();
            state.league_client_connected = false;
            drop(state);
            sleep(Duration::from_secs(5)).await;
            continue;
        };

        let client = match crate::services::lcu_client::LcuClient::new(credentials) {
            Ok(client) => client,
            Err(error) => {
                logger::error(
                    "agent.collector",
                    &format!("Failed creating LCU client: {error}"),
                );
                let mut state = runtime.write().await;
                state.collector_state = "backoff".to_string();
                state.league_client_connected = false;
                drop(state);
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let (detected_patch, patch_changed) =
            crate::services::patch_manager::ensure_patch_freshness(Some(&client)).await;
        if patch_changed {
            let db_path = path.clone();
            let global_path = global_aram_database_path();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(conn) = open_connection(&db_path) {
                    let _ = invalidate_statistics_projections(&conn);
                }
                if let Ok(conn) = open_connection(&global_path) {
                    let _ = invalidate_statistics_projections(&conn);
                }
            })
            .await;
        }

        {
            let mut state = runtime.write().await;
            if state.collector_state != "event_pending" {
                state.collector_state = "collecting".to_string();
            }
            state.league_client_connected = true;
            if detected_patch.is_some() {
                state.current_patch = detected_patch;
            }
        }

        let puuid = client
            .get_json::<Value>("/lol-summoner/v1/current-summoner")
            .await
            .ok()
            .and_then(|summoner| {
                summoner
                    .get("puuid")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
        // Version this cursor so accounts scanned with the old current-summoner
        // route are re-scanned through the PUUID history endpoint.
        let backfill_key = "match_history_backfill_v2_puuid";
        let backfill_complete = sync_state_value(&path, backfill_key)
            .ok()
            .flatten()
            .as_deref()
            == puuid.as_deref();

        // Track whether summoner identity changed or event was queued
        let puuid_changed = puuid.is_some() && puuid != last_reconciled_puuid;
        let needs_reconciliation = {
            let mut state = runtime.write().await;
            if state.collector_state == "event_pending" || puuid_changed {
                state.collector_state = "collecting".to_string();
                true
            } else {
                false
            }
        };

        let mut new_personal_games_found = false;
        if !backfill_complete {
            logger::info(
                "agent.collector",
                &format!(
                    "Collector account scan status: puuid_present={}, backfill_complete=false",
                    puuid.is_some()
                ),
            );
            match backfill_match_history(&client, &path, puuid.as_deref(), &runtime).await {
                Ok(count) => {
                    logger::info(
                        "agent.collector",
                        &format!("History backfill complete: {count} records discovered"),
                    );
                    if count > 0 {
                        new_personal_games_found = true;
                    }
                    if let Some(puuid) = puuid.as_deref() {
                        if let Err(error) = set_sync_state_value(&path, backfill_key, puuid) {
                            logger::error(
                                "agent.collector",
                                &format!("Failed recording history backfill state: {error}"),
                            );
                        }
                    }
                    last_reconciled_puuid = puuid.clone();
                }
                Err(error) => logger::error(
                    "agent.collector",
                    &format!("History backfill failed: {error}"),
                ),
            }
        } else if needs_reconciliation {
            let route = if let Some(puuid) = puuid.as_deref() {
                format!("/lol-match-history/v1/products/lol/{puuid}/matches?begIndex=0&endIndex=19")
            } else {
                "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=19"
                    .to_string()
            };
            match client.get_json::<Value>(&route).await {
                Ok(history) => match store_recent_games(&path, &history, puuid.as_deref()) {
                    Ok((count, has_new)) => {
                        last_reconciled_puuid = puuid.clone();
                        if has_new {
                            new_personal_games_found = true;
                            logger::info(
                                "agent.collector",
                                &format!("Reconciled {count} recent games (new games discovered)"),
                            );
                        } else {
                            logger::debug(
                                "agent.collector",
                                &format!("Reconciled {count} recent games (no new games)"),
                            );
                        }
                    }
                    Err(error) => logger::error(
                        "agent.collector",
                        &format!("Failed storing recent games: {error}"),
                    ),
                },
                Err(error) => logger::error(
                    "agent.collector",
                    &format!("Recent history reconciliation failed: {error}"),
                ),
            }
        }

        if let Some(puuid) = puuid.as_deref() {
            let should_start = {
                let mut state = runtime.write().await;
                if state.global_collection_running {
                    false
                } else {
                    state.global_collection_running = true;
                    true
                }
            };
            if should_start {
                let client = client.clone();
                let puuid = puuid.to_string();
                let runtime = runtime.clone();
                let generation = CRAWLER_GENERATION.load(std::sync::atomic::Ordering::SeqCst);
                tokio::spawn(async move {
                    if let Err(error) =
                        collect_global_aram(&client, &puuid, generation, &runtime).await
                    {
                        logger::error(
                            "agent.global-aram",
                            &format!("Global ARAM collection failed: {error}"),
                        );
                    }
                    if CRAWLER_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
                        return;
                    }
                    let mut state = runtime.write().await;
                    state.global_collection_running = false;
                    state
                        .operations
                        .retain(|operation| operation.id != "global-database");
                    drop(state);
                    refresh_statistics_projections(
                        database_path(),
                        global_aram_database_path(),
                        runtime.clone(),
                    )
                    .await;
                });
            } else if new_personal_games_found {
                let should_refresh = !runtime.read().await.projection_running;
                if should_refresh {
                    tokio::spawn(refresh_statistics_projections(
                        path.clone(),
                        global_aram_database_path(),
                        runtime.clone(),
                    ));
                }
            }
        }

        {
            let mut state = runtime.write().await;
            if state.collector_state != "event_pending"
                && !state.collector_state.starts_with("game_in_progress")
            {
                state.collector_state = "live".to_string();
            }
        }
        tokio::select! {
            _ = COLLECTOR_WAKEUP.notified() => {
                logger::debug("agent.collector", "Reactive wakeup triggered by LCU event");
            }
            _ = sleep(Duration::from_secs(60)) => {}
        }
    }
}

pub async fn websocket_loop(runtime: Arc<RwLock<RuntimeState>>) {
    loop {
        let Some(credentials) = crate::services::lcu_client::LcuClient::find_credentials() else {
            sleep(Duration::from_secs(5)).await;
            continue;
        };

        let url = format!("wss://127.0.0.1:{}/", credentials.port);
        let mut request = match url.clone().into_client_request() {
            Ok(request) => request,
            Err(error) => {
                logger::error("agent.websocket", &format!("Invalid request: {error}"));
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        let Ok(auth) = HeaderValue::from_str(&credentials.basic_auth_header()) else {
            logger::error("agent.websocket", "Invalid authorization header");
            sleep(Duration::from_secs(5)).await;
            continue;
        };
        request.headers_mut().insert("Authorization", auth);

        let tls = match TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
        {
            Ok(tls) => tls,
            Err(error) => {
                logger::error(
                    "agent.websocket",
                    &format!("Failed creating TLS connector: {error}"),
                );
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let connected = tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            Some(Connector::NativeTls(tls)),
        )
        .await;
        let Ok((stream, _)) = connected else {
            sleep(Duration::from_secs(5)).await;
            continue;
        };

        let (mut writer, mut reader) = stream.split();
        if writer
            .send(Message::Text(r#"[5, "OnJsonApiEvent"]"#.to_string().into()))
            .await
            .is_err()
        {
            sleep(Duration::from_secs(5)).await;
            continue;
        }

        logger::info("agent.websocket", "Connected to LCU websocket");
        while let Some(Ok(message)) = reader.next().await {
            let Message::Text(text) = message else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let payload = value.as_array().and_then(|array| array.get(2));
            let uri = payload.and_then(|p| p.get("uri")).and_then(Value::as_str);
            let data = payload.and_then(|p| p.get("data"));

            if let Some(uri) = uri {
                if uri == "/lol-gameflow/v1/gameflow-phase" {
                    let phase = data.and_then(Value::as_str).unwrap_or("");
                    let mut state = runtime.write().await;
                    if phase == "InProgress" {
                        state.collector_state = "game_in_progress".to_string();
                        logger::info(
                            "agent.websocket",
                            "Gameflow phase InProgress: pausing background crawler",
                        );
                    } else if phase == "EndOfGame" || phase == "WaitingForStats" {
                        state.collector_state = "event_pending".to_string();
                        COLLECTOR_WAKEUP.notify_one();
                    } else if state.collector_state == "game_in_progress" {
                        state.collector_state = "live".to_string();
                        COLLECTOR_WAKEUP.notify_one();
                    }
                } else if uri.contains("match-history") || uri == "/lol-login/v1/session" {
                    let mut state = runtime.write().await;
                    state.collector_state = "event_pending".to_string();
                    COLLECTOR_WAKEUP.notify_one();
                }
            }
        }

        logger::warn("agent.websocket", "Disconnected from LCU websocket");
        sleep(Duration::from_secs(2)).await;
    }
}

pub async fn run_process() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let port = args
        .next()
        .filter(|argument| argument == "--port")
        .and_then(|_| args.next())
        .ok_or_else(|| "agent requires --port <port>".to_string())?
        .parse::<u16>()
        .map_err(|error| format!("invalid agent port: {error}"))?;
    let token = args
        .next()
        .filter(|argument| argument == "--token")
        .and_then(|_| args.next())
        .ok_or_else(|| "agent requires --token <token>".to_string())?;

    let path = database_path();
    initialize_database(&path)?;
    let runtime = Arc::new(RwLock::new(RuntimeState {
        collector_state: "sleeping".to_string(),
        ..RuntimeState::default()
    }));
    tokio::spawn(collector_loop(path.clone(), runtime.clone()));
    tokio::spawn(websocket_loop(runtime.clone()));

    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|error| format!("failed binding agent IPC listener: {error}"))?;

    logger::info(
        "agent.process",
        &format!(
            "Listening on agent IPC port {} with database {:?}",
            listener
                .local_addr()
                .map_err(|error| format!("failed reading agent address: {error}"))?
                .port(),
            path
        ),
    );

    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|error| format!("agent IPC accept failed: {error}"))?;
        let token = token.clone();
        let path = path.clone();
        let runtime = runtime.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = stream.into_split();
            let mut lines = BufReader::new(reader).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let response = match serde_json::from_str::<AgentRequest>(&line) {
                    Ok(request) => handle_request(request, &token, &path, &runtime).await,
                    Err(error) => response_error(format!("invalid request: {error}")),
                };

                let mut encoded = match serde_json::to_vec(&response) {
                    Ok(value) => value,
                    Err(error) => {
                        response_error(format!("failed encoding response: {error}"));
                        Vec::new()
                    }
                };
                encoded.push(b'\n');
                if writer.write_all(&encoded).await.is_err() {
                    break;
                }
            }
        });
    }
}

pub fn run_process_from_main() -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| format!("failed creating agent runtime: {error}"))?;
    runtime.block_on(run_process())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacer_initialization() {
        let pacer_100 = WorkerRatePacer::new(100);
        assert_eq!(pacer_100.interval, Duration::from_millis(600));

        let pacer_60 = WorkerRatePacer::new(60);
        assert_eq!(pacer_60.interval, Duration::from_millis(1000));
    }

    #[tokio::test]
    async fn test_pacer_spacing_fast_requests() {
        let mut pacer = WorkerRatePacer {
            interval: Duration::from_millis(40),
            last_request_start: None,
        };

        let t0 = std::time::Instant::now();
        pacer.pace_next().await;
        // Simulate very fast request (e.g. 5ms)
        tokio::time::sleep(Duration::from_millis(5)).await;
        pacer.pace_next().await;
        let elapsed = t0.elapsed();

        // Total time should be at least 35ms (accounting for timer granularity)
        assert!(
            elapsed >= Duration::from_millis(35),
            "Expected at least 35ms, got {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_pacer_slow_request_not_bottlenecked() {
        let mut pacer = WorkerRatePacer {
            interval: Duration::from_millis(40),
            last_request_start: None,
        };

        pacer.pace_next().await;
        // Simulate a slow request that exceeds the interval (60ms > 40ms)
        tokio::time::sleep(Duration::from_millis(60)).await;

        let before = std::time::Instant::now();
        // Should return almost immediately without sleeping another 40ms
        pacer.pace_next().await;
        let wait_time = before.elapsed();

        assert!(
            wait_time < Duration::from_millis(15),
            "Expected immediate release after slow request, but waited {:?}",
            wait_time
        );
    }

    #[tokio::test]
    async fn test_pacer_anti_bursting_after_slow_request() {
        let mut pacer = WorkerRatePacer {
            interval: Duration::from_millis(40),
            last_request_start: None,
        };

        pacer.pace_next().await;
        // Request 1 takes 60ms (slow)
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Request 2 released immediately
        pacer.pace_next().await;

        // Request 2 takes 5ms (fast)
        tokio::time::sleep(Duration::from_millis(5)).await;

        let before_r3 = std::time::Instant::now();
        // Request 3 MUST be spaced from Request 2's start by 40ms
        pacer.pace_next().await;
        let r3_wait = before_r3.elapsed();

        assert!(
            r3_wait >= Duration::from_millis(25),
            "Expected Request 3 to be spaced from Request 2 start, but waited only {:?}",
            r3_wait
        );
    }

    #[test]
    fn test_circuit_breaker_rate_limit_and_error() {
        let mut cb = CrawlerCircuitBreaker::new();
        assert!(cb.is_in_cooldown().is_none());

        // Normal errors don't trigger backoff until 4 consecutive
        assert!(cb.record_error().is_none());
        assert!(cb.record_error().is_none());
        assert!(cb.record_error().is_none());
        assert!(cb.is_in_cooldown().is_none());

        // 4th error triggers cooldown (initial backoff 2s, next 4s)
        let action = cb.record_error();
        assert_eq!(action, Some((4, 2, 4)));
        assert!(cb.is_in_cooldown().is_some());

        // Concurrent in-flight request failing while cooldown is ALREADY active must NOT escalate backoff!
        let stale_req_start = std::time::Instant::now() - Duration::from_secs(5);
        let joined = cb.record_failure_for_request(false, None, stale_req_start);
        assert!(matches!(joined, CooldownOutcome::JoinedExisting { .. }));
        assert_eq!(cb.consecutive_errors, 4);
        assert_eq!(cb.current_backoff_secs, 4);

        // Success resets streak
        cb.record_success();
        assert!(cb.is_in_cooldown().is_none());
        assert_eq!(cb.consecutive_errors, 0);

        // HTTP 429 triggers immediately on first error and respects Retry-After if provided
        let cooldown = cb.record_rate_limit(Some(Duration::from_secs(5)));
        assert_eq!(cooldown, 5);
        assert!(cb.is_in_cooldown().is_some());
    }

    #[test]
    fn test_adaptive_rate_limiter_aimd_and_ceiling_locking() {
        let mut limiter = AdaptiveRateLimiter::new(60, 20, 100);
        assert_eq!(limiter.current_rpm(), 60);
        assert_eq!(limiter.ceiling_rpm(), None);

        // 7 successes - not yet threshold (8)
        for _ in 0..7 {
            assert!(limiter.record_success().is_none());
        }
        assert_eq!(limiter.current_rpm(), 60);

        // 8th success increases RPM additively (+2 when no ceiling is learned yet)
        assert_eq!(limiter.record_success(), Some(62));
        assert_eq!(limiter.current_rpm(), 62);

        // Multiplicative decrease on transient error (-25%) and learns safe ceiling (85% of 62 = 52)
        let r1_start = std::time::Instant::now();
        let decreased = limiter.record_backoff_for_request(false, r1_start);
        assert_eq!(decreased, Some(46));
        assert_eq!(limiter.current_rpm(), 46);
        assert_eq!(limiter.ceiling_rpm(), Some(52));

        // Stale concurrent request that started at or before the backoff must be ignored
        assert_eq!(limiter.record_backoff_for_request(false, r1_start), None);
        assert_eq!(limiter.current_rpm(), 46);

        // Stepping up toward learned ceiling (52) uses +1 RPM per 8 successes and locks at 52
        for expected in 47..=52 {
            for _ in 0..7 {
                assert!(limiter.record_success().is_none());
            }
            assert_eq!(limiter.record_success(), Some(expected));
        }
        assert_eq!(limiter.current_rpm(), 52);

        // Once at the learned ceiling (52), 8 successes do NOT increase RPM (it holds steady)
        for _ in 0..16 {
            assert!(limiter.record_success().is_none());
        }
        assert_eq!(limiter.current_rpm(), 52);

        // Multiplicative decrease on 429 (-50%)
        let rate_limited = limiter.record_backoff(true);
        assert_eq!(rate_limited, 26);
        assert_eq!(limiter.current_rpm(), 26);

        // Further backoff clamped to min_rpm (20)
        let clamped = limiter.record_backoff(true);
        assert_eq!(clamped, 20);
        assert_eq!(limiter.current_rpm(), 20);
    }

    #[tokio::test]
    async fn test_adaptive_rate_limiter_burst_and_spacing() {
        let mut limiter = AdaptiveRateLimiter::with_burst(600, 60, 600, None, 20, 3, 5, 16);
        assert_eq!(limiter.current_rpm(), 600);

        let t0 = std::time::Instant::now();
        // 1st request
        limiter.pace_next().await;
        // 2nd request - must be spaced at least 20ms
        limiter.pace_next().await;
        let t2 = t0.elapsed();
        assert!(t2 >= Duration::from_millis(18));

        // 3rd request - reaches burst limit of 3
        limiter.pace_next().await;
        assert_eq!(limiter.burst_window.len(), 3);
    }
}
