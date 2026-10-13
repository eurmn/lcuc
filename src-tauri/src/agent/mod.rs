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
    connection.execute_batch("PRAGMA synchronous = NORMAL; PRAGMA temp_store = MEMORY;")?;
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
    Ok(())
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

#[derive(Debug, Default)]
pub struct RuntimeState {
    pub collector_state: String,
    pub league_client_connected: bool,
    pub queued_jobs: usize,
    pub global_collection_running: bool,
    pub projection_running: bool,
    pub operations: Vec<ProgressOperation>,
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
    let effective_baseline = if kind == "champion" {
        baseline
    } else {
        champion_baseline
    };
    json!({
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
        "win_rate": aggregate.wins as f64 / games,
        "wpa": aggregate.wins as f64 / games - effective_baseline,
        "pick_rate": aggregate.games as f64 / (population_count.max(1) as f64),
        "damage_per_minute": aggregate.damage as f64 / (duration / 60.0),
        "champions": champion_rows,
        "partners": partner_rows,
    })
}

fn load_statistics_projection(
    path: &PathBuf,
    kind: &str,
) -> Result<Option<(i64, f64, Vec<Value>)>, String> {
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
    Ok(row)
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
    let rows_json = serde_json::to_string(rows)
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
        .map_err(|error| format!("failed storing {kind} statistics projection: {error}"))
}

fn render_statistics_response(
    args: &Value,
    scope: &str,
    kind: &str,
    sample_count: i64,
    baseline: f64,
    mut rows: Vec<Value>,
) -> Value {
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
                        .fold(None, |acc: Option<f64>, v| Some(acc.map_or(v, |m| m.max(v))))
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
    let scope = args
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("personal");
    let kind = args
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("champion");
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
                            .fold(None, |acc: Option<f64>, v| Some(acc.map_or(v, |m| m.max(v))))
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
    let mut cache_args = args.clone();
    if let Some(object) = cache_args.as_object_mut() {
        object.remove("offset");
        object.remove("limit");
    }
    let cache_key = cache_args.to_string();
    if let Ok(mut cache) = STATISTICS_QUERY_CACHE.lock() {
        if let Some((created_at, cached)) = cache.get(&cache_key) {
            if created_at.elapsed() < Duration::from_secs(30) {
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
                return Ok(json!({
                    "scope": scope,
                    "kind": kind,
                    "sample_count": cached.get("sample_count").and_then(Value::as_i64).unwrap_or(0),
                    "baseline": cached.get("baseline").and_then(Value::as_f64).unwrap_or(0.0),
                    "total": total,
                    "offset": offset,
                    "limit": limit,
                    "rows": rows,
                    "cached": true
                }));
            }
        }
        cache.retain(|_, (created_at, _)| created_at.elapsed() < Duration::from_secs(120));
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
            "combo" => fact.item_ids.len() + fact.augment_ids.len() > 1,
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
    let mut aggregates: HashMap<String, (String, Aggregate)> = HashMap::new();
    for fact in &facts {
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
                let mut parts = fact
                    .item_ids
                    .iter()
                    .filter(|id| !PORO_SNAX_IDS.contains(id))
                    .map(|id| format!("i{id}"))
                    .chain(fact.augment_ids.iter().map(|id| format!("a{id}")))
                    .collect::<Vec<_>>();
                parts.sort();
                if parts.len() > 1 {
                    vec![(
                        format!("combo:{}", parts.join("+")),
                        parts
                            .iter()
                            .map(|part| part.trim_start_matches(&['i', 'a'][..]))
                            .collect::<Vec<_>>()
                            .join(" + "),
                    )]
                } else {
                    Vec::new()
                }
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
    let mut rows = aggregates
        .into_iter()
        .filter_map(|(id, (name, aggregate))| {
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
                        .fold(None, |acc: Option<f64>, v| Some(acc.map_or(v, |m| m.max(v))))
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
        store_statistics_projection(
            if scope == "global" {
                global_database
            } else {
                database
            },
            kind,
            sample_count,
            baseline,
            &rows,
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
        "#,
        )
        .map_err(|error| format!("failed initializing global ARAM database: {error}"))?;
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
    Ok(())
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
            let connection = match open_connection(path) {
                Ok(connection) => connection,
                Err(error) => {
                    return response_error(format!("failed opening agent database: {error}"))
                }
            };
            let mut statement = match connection.prepare(
                "SELECT game_id, participant_id, puuid, champion_id, win,
                        item_ids_json, augment_ids_json, kills, deaths, assists, damage, gold
                 FROM game_build_facts ORDER BY game_id DESC LIMIT 500",
            ) {
                Ok(statement) => statement,
                Err(error) => {
                    return response_error(format!("failed preparing build statistics: {error}"))
                }
            };
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
            });
            let rows = match rows {
                Ok(rows) => rows,
                Err(error) => {
                    return response_error(format!("failed reading build statistics: {error}"))
                }
            };
            let facts: Result<Vec<Value>, _> = rows.collect();
            match facts {
                Ok(facts) => response_ok(json!({ "facts": facts })),
                Err(error) => {
                    response_error(format!("failed collecting build statistics: {error}"))
                }
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
                ))
            })
            .await;
            let (limit, games, queued_accounts, workers) = match config {
                Ok(Ok(values)) => values,
                Ok(Err(error)) => return response_error(error),
                Err(error) => {
                    return response_error(format!("global config worker failed: {error}"))
                }
            };
            let state = runtime.read().await;
            response_ok(json!({
                "limit": limit,
                "games": games,
                "workers": workers,
                "database_path": global_path,
                "collector_state": state.collector_state,
                "queued_jobs": queued_accounts,
                "operations": state.operations.iter().map(|operation| json!({
                    "id": operation.id,
                    "message": operation.message,
                    "current": operation.current,
                    "total": operation.total
                })).collect::<Vec<_>>()
            }))
        }
        "set_global_aram_limit" => {
            let global_path = global_aram_database_path();
            if let Err(error) = initialize_global_aram_database(&global_path) {
                return response_error(error);
            }
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(1000) as usize;
            match set_global_game_limit(&global_path, limit) {
                Ok(()) => {
                    let games = global_game_count(&global_path).unwrap_or(0);
                    let mut state = runtime.write().await;
                    state.collector_state = if games >= limit {
                        format!("global-aram:{games}/{limit} (paused)")
                    } else {
                        format!("global-aram:{games}/{limit} (starting)")
                    };
                    response_ok(json!({ "limit": limit }))
                }
                Err(error) => response_error(error),
            }
        }
        "set_global_aram_workers" => {
            let global_path = global_aram_database_path();
            if let Err(error) = initialize_global_aram_database(&global_path) {
                return response_error(error);
            }
            let workers = args.get("workers").and_then(Value::as_u64).unwrap_or(3) as usize;
            match set_global_worker_count(&global_path, workers) {
                Ok(()) => response_ok(json!({ "workers": workers.clamp(1, 16) })),
                Err(error) => response_error(error),
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
        transaction
            .execute(
                "INSERT INTO games (
                    game_id, puuid, queue_id, map_id, game_mode,
                    game_creation, game_duration, collection_status, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'discovered', CURRENT_TIMESTAMP)
                ON CONFLICT(game_id) DO UPDATE SET
                    puuid = COALESCE(excluded.puuid, games.puuid),
                    queue_id = excluded.queue_id,
                    map_id = excluded.map_id,
                    game_mode = excluded.game_mode,
                    game_creation = excluded.game_creation,
                    game_duration = excluded.game_duration,
                    updated_at = CURRENT_TIMESTAMP",
                rusqlite::params![
                    game_id,
                    puuid,
                    game.get("queueId").and_then(Value::as_i64),
                    game.get("mapId").and_then(Value::as_i64),
                    game.get("gameMode").and_then(Value::as_str),
                    game.get("gameCreation").and_then(Value::as_i64),
                    game.get("gameDuration").and_then(Value::as_i64),
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

fn store_global_detail(
    path: &PathBuf,
    source_puuid: &str,
    detail: &Value,
) -> Result<Vec<String>, String> {
    let game_id = detail
        .get("gameId")
        .and_then(Value::as_i64)
        .ok_or_else(|| "global ARAM detail did not contain gameId".to_string())?;
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global ARAM database for detail: {error}"))?;
    let game_exists = connection
        .query_row(
            "SELECT 1 FROM aram_games WHERE game_id = ?1 LIMIT 1",
            rusqlite::params![game_id],
            |_| Ok(()),
        )
        .is_ok();
    if game_exists {
        return Ok(Vec::new());
    }
    let detail_json = serde_json::to_string(detail)
        .map_err(|error| format!("failed encoding global ARAM detail {game_id}: {error}"))?;
    connection
        .execute(
            "INSERT OR IGNORE INTO aram_games (
            game_id, source_puuid, queue_id, map_id, game_mode,
            game_creation, game_duration, detail_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
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
    let mut discovered = Vec::new();
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
            discovered.push(puuid.to_string());
            queue_global_account(path, puuid)?;
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
        connection
            .execute(
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
                format!(
                    "failed storing global ARAM participant {game_id}:{participant_id}: {error}"
                )
            })?;
    }
    if !game_exists {
        invalidate_statistics_projections(&connection)?;
    }
    Ok(discovered)
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
    /// If the previous request took longer than `interval`, it does not delay, allowing immediate execution
    /// without accumulating artificial lag. It then records the new request start time so subsequent requests
    /// maintain the safe spacing interval (preventing request bursts/spam).
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

struct CrawlerCircuitBreaker {
    consecutive_errors: u32,
    cooldown_until: Option<std::time::Instant>,
    current_backoff_secs: u64,
}

impl CrawlerCircuitBreaker {
    fn new() -> Self {
        Self {
            consecutive_errors: 0,
            cooldown_until: None,
            current_backoff_secs: 120, // 2 minutes initial backoff
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

    fn record_rate_limit(&mut self, cooldown_secs: u64) -> u64 {
        self.consecutive_errors += 1;
        let cooldown = cooldown_secs.max(self.current_backoff_secs);
        self.cooldown_until = Some(std::time::Instant::now() + Duration::from_secs(cooldown));
        self.current_backoff_secs = (cooldown * 2).min(1800);
        cooldown
    }

    fn record_error(&mut self) -> Option<(u32, u64, u64)> {
        self.consecutive_errors += 1;
        if self.consecutive_errors >= 4 {
            let cooldown = self.current_backoff_secs;
            self.cooldown_until = Some(std::time::Instant::now() + Duration::from_secs(cooldown));
            let next_backoff = (self.current_backoff_secs * 2).min(1800);
            self.current_backoff_secs = next_backoff;
            Some((self.consecutive_errors, cooldown, next_backoff))
        } else {
            None
        }
    }

    fn record_success(&mut self) {
        if self.consecutive_errors > 0 {
            self.consecutive_errors = 0;
            self.current_backoff_secs = 120;
            self.cooldown_until = None;
        }
    }
}

async fn run_global_aram_worker(
    worker_id: usize,
    client: crate::services::lcu_client::LcuClient,
    path: PathBuf,
    runtime: Arc<RwLock<RuntimeState>>,
    in_flight_games: Arc<tokio::sync::Mutex<HashSet<i64>>>,
    circuit_breaker: Arc<tokio::sync::Mutex<CrawlerCircuitBreaker>>,
) {
    logger::info(
        "agent.global-aram",
        &format!("Global ARAM worker #{worker_id} started"),
    );
    let mut empty_streak = 0;
    let mut pacer = WorkerRatePacer::new(100);

    loop {
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

        // Pace: max 100 requests/minute per worker, evenly spaced, non-bottlenecked
        pacer.pace_next().await;

        let route = format!(
            "/lol-match-history/v1/products/lol/{account_puuid}/matches?begIndex=0&endIndex=19"
        );
        let history = match client.get_json::<Value>(&route).await {
            Ok(history) => {
                circuit_breaker.lock().await.record_success();
                history
            }
            Err(error) => {
                logger::warn(
                    "agent.global-aram",
                    &format!("Worker #{worker_id}: Match history unavailable for account {account_puuid}: {error}"),
                );
                let mark_path = path.clone();
                let mark_puuid = account_puuid.clone();

                let is_rate_limited = error.contains("429");
                let cooldown_action = {
                    let mut cb = circuit_breaker.lock().await;
                    if is_rate_limited {
                        let cooldown = cb.record_rate_limit(60);
                        Some((cb.consecutive_errors, cooldown, cb.current_backoff_secs))
                    } else {
                        cb.record_error()
                    }
                };

                if let Some((streak, cooldown_secs, next_backoff)) = cooldown_action {
                    let mins = (cooldown_secs + 59) / 60;
                    let next_mins = (next_backoff + 59) / 60;
                    let reason = if is_rate_limited { "HTTP 429 Rate Limit" } else { "unavailable consecutively" };
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
                } else {
                    let _ = tokio::task::spawn_blocking(move || {
                        mark_global_account(&mark_path, &mark_puuid, "failed")
                    })
                    .await;
                }
                continue;
            }
        };

        let summaries = history
            .get("games")
            .and_then(|value| value.get("games"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let aram_summaries: Vec<Value> = summaries.into_iter().filter(is_aram_mayhem).collect();

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

        for game_id in games_to_fetch {
            // Check if the circuit breaker cooldown is active before starting each game request
            let wait_dur = {
                let cb = circuit_breaker.lock().await;
                cb.is_in_cooldown()
            };
            if let Some(dur) = wait_dur {
                in_flight_games.lock().await.remove(&game_id);
                tokio::time::sleep(dur).await;
                break;
            }

            if let Ok(count) = global_game_count(&path) {
                if let Ok(limit) = global_game_limit(&path) {
                    if count >= limit {
                        in_flight_games.lock().await.remove(&game_id);
                        break;
                    }
                }
            }

            // Enforce max 100 games/minute limit per worker, evenly spaced, non-bottlenecked
            pacer.pace_next().await;

            let result = client
                .get_json::<Value>(&format!("/lol-match-history/v1/games/{game_id}"))
                .await;

            match result {
                Ok(detail) if is_aram_mayhem(&detail) => {
                    circuit_breaker.lock().await.record_success();
                    let write_path = path.clone();
                    let write_source = account_puuid.clone();
                    let store_result = tokio::task::spawn_blocking(move || {
                        store_global_detail(&write_path, &write_source, &detail)
                    })
                    .await;
                    if let Ok(Ok(_)) = store_result {
                        if let (Ok(count), Ok(live_limit)) =
                            (global_game_count(&path), global_game_limit(&path))
                        {
                            let mut state = runtime.write().await;
                            state.collector_state = format!("global-aram:{count}/{live_limit}");
                            if let Some(operation) = state
                                .operations
                                .iter_mut()
                                .find(|operation| operation.id == "global-database")
                            {
                                operation.current = Some(count);
                                operation.total = Some(live_limit);
                                operation.message = "Updating global game database".to_string();
                            }
                            logger::info(
                                "agent.global-aram",
                                &format!("Worker #{worker_id} stored game {game_id}; dataset is {count}/{live_limit}"),
                            );
                            if count >= live_limit {
                                in_flight_games.lock().await.remove(&game_id);
                                break;
                            }
                        }
                    }
                }
                Ok(_) => {
                    circuit_breaker.lock().await.record_success();
                }
                Err(error) => {
                    logger::warn(
                        "agent.global-aram",
                        &format!("Worker #{worker_id} failed fetching game {game_id}: {error}"),
                    );

                    let is_rate_limited = error.contains("429");
                    let cooldown_action = {
                        let mut cb = circuit_breaker.lock().await;
                        if is_rate_limited {
                            let cooldown = cb.record_rate_limit(60);
                            Some((cb.consecutive_errors, cooldown, cb.current_backoff_secs))
                        } else if error.contains("timed out") || error.contains("503") || error.contains("504") {
                            cb.record_error()
                        } else {
                            None
                        }
                    };

                    if let Some((streak, cooldown_secs, next_backoff)) = cooldown_action {
                        let mins = (cooldown_secs + 59) / 60;
                        let next_mins = (next_backoff + 59) / 60;
                        let reason = if is_rate_limited { "HTTP 429 Rate Limit" } else { "consecutive timeouts/errors" };
                        logger::warn(
                            "agent.global-aram",
                            &format!(
                                "Game fetch {reason} ({streak} errors). Backing off all workers for {cooldown_secs}s before retrying (next backoff: {next_mins}m)..."
                            ),
                        );
                        {
                            let mut state = runtime.write().await;
                            state.collector_state = format!("global-aram:paused (backoff {mins}m)");
                        }
                        in_flight_games.lock().await.remove(&game_id);
                        tokio::time::sleep(Duration::from_secs(cooldown_secs)).await;
                        break;
                    }
                }
            }
            in_flight_games.lock().await.remove(&game_id);
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
    runtime: &Arc<RwLock<RuntimeState>>,
) -> Result<(), String> {
    let path = global_aram_database_path();
    let initialize_path = path.clone();
    tokio::task::spawn_blocking(move || initialize_global_aram_database(&initialize_path))
        .await
        .map_err(|error| format!("global database initialization failed: {error}"))??;
    let current = global_game_count(&path)?;
    let limit = global_game_limit(&path)?;
    let worker_count = global_worker_count(&path).unwrap_or(3);
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

    logger::info(
        "agent.global-aram",
        &format!(
            "Starting global ARAM collection with {worker_count} concurrent workers at {}/{} games",
            current, limit
        ),
    );

    let in_flight_games = Arc::new(tokio::sync::Mutex::new(HashSet::new()));
    let circuit_breaker = Arc::new(tokio::sync::Mutex::new(CrawlerCircuitBreaker::new()));
    let mut handles = Vec::new();

    for worker_id in 0..worker_count {
        let client = client.clone();
        let path = path.clone();
        let runtime = runtime.clone();
        let in_flight = in_flight_games.clone();
        let cb = circuit_breaker.clone();
        handles.push(tokio::spawn(async move {
            run_global_aram_worker(worker_id, client, path, runtime, in_flight, cb).await;
        }));
    }

    for handle in handles {
        let _ = handle.await;
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

fn global_worker_count(path: &PathBuf) -> Result<usize, String> {
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global ARAM config: {error}"))?;
    let val: Option<String> = connection
        .query_row(
            "SELECT value FROM config WHERE key = 'worker_count'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("failed reading global ARAM worker count: {error}"))?;
    let count = val
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(3)
        .clamp(1, 16);
    Ok(count)
}

fn set_global_worker_count(path: &PathBuf, count: usize) -> Result<(), String> {
    let count = count.clamp(1, 16);
    let connection = open_connection(path)
        .map_err(|error| format!("failed opening global ARAM config: {error}"))?;
    connection
        .execute(
            "INSERT INTO config (key, value, updated_at) VALUES ('worker_count', ?1, CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            rusqlite::params![count.to_string()],
        )
        .map(|_| ())
        .map_err(|error| format!("failed storing global ARAM worker count: {error}"))
}

async fn request_delay() {
    let millis = 500
        + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| (d.subsec_nanos() % 501) as u64)
            .unwrap_or(250));
    sleep(Duration::from_millis(millis)).await;
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

        {
            let mut state = runtime.write().await;
            if state.collector_state != "event_pending" {
                state.collector_state = "collecting".to_string();
            }
            state.league_client_connected = true;
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
                tokio::spawn(async move {
                    if let Err(error) = collect_global_aram(&client, &puuid, &runtime).await {
                        logger::error(
                            "agent.global-aram",
                            &format!("Global ARAM collection failed: {error}"),
                        );
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
            if state.collector_state != "event_pending" {
                state.collector_state = "live".to_string();
            }
        }
        sleep(Duration::from_secs(15)).await;
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
            let Some(uri) = value
                .as_array()
                .and_then(|array| array.get(2))
                .and_then(|payload| payload.get("uri"))
                .and_then(Value::as_str)
            else {
                continue;
            };

            if uri.contains("match-history")
                || uri == "/lol-gameflow/v1/gameflow-phase"
                || uri == "/lol-login/v1/session"
            {
                let mut state = runtime.write().await;
                state.collector_state = "event_pending".to_string();
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

        // 4th error triggers cooldown
        let action = cb.record_error();
        assert!(action.is_some());
        assert!(cb.is_in_cooldown().is_some());

        // Success resets streak
        cb.record_success();
        assert!(cb.is_in_cooldown().is_none());
        assert_eq!(cb.consecutive_errors, 0);

        // HTTP 429 triggers immediately on first error
        let cooldown = cb.record_rate_limit(60);
        assert_eq!(cooldown, 120); // max of 60 and initial backoff 120
        assert!(cb.is_in_cooldown().is_some());
    }
}
