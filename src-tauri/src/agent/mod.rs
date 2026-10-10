use futures_util::{SinkExt, StreamExt};
use native_tls::TlsConnector;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::Connector;
use crate::logger;

#[derive(Debug, Deserialize)]
struct AgentRequest {
    token: String,
    command: String,
}

#[derive(Debug, Serialize)]
struct AgentResponse {
    ok: bool,
    data: Value,
    error: Option<String>,
}

#[derive(Debug, Default)]
struct RuntimeState {
    collector_state: String,
    league_client_connected: bool,
    queued_jobs: usize,
}

fn database_path() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu-companion")
        .join("agent.db")
}

fn legacy_database_path() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu_companion")
        .join("lcu_companion.db")
}

fn initialize_database(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed creating agent database directory: {error}"))?;
    }

    let connection = Connection::open(path)
        .map_err(|error| format!("Failed opening agent database: {error}"))?;

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
        .map_err(|error| format!("Failed initializing agent database schema: {error}"))
        .and_then(|_| import_legacy_store(&connection, path))
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

    match request.command.as_str() {
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
            }))
        }
        "build_statistics" => {
            let connection = match Connection::open(path) {
                Ok(connection) => connection,
                Err(error) => return response_error(format!("failed opening agent database: {error}")),
            };
            let mut statement = match connection.prepare(
                "SELECT game_id, participant_id, puuid, champion_id, win,
                        item_ids_json, augment_ids_json, kills, deaths, assists, damage, gold
                 FROM game_build_facts ORDER BY game_id DESC LIMIT 500",
            ) {
                Ok(statement) => statement,
                Err(error) => return response_error(format!("failed preparing build statistics: {error}")),
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
                Err(error) => return response_error(format!("failed reading build statistics: {error}")),
            };
            let facts: Result<Vec<Value>, _> = rows.collect();
            match facts {
                Ok(facts) => response_ok(json!({ "facts": facts })),
                Err(error) => response_error(format!("failed collecting build statistics: {error}")),
            }
        }
        "shutdown" => response_ok(json!({ "accepted": true })),
        command => response_error(format!("unknown command: {command}")),
    }
}

fn store_recent_games(path: &PathBuf, raw_history: &Value, puuid: Option<&str>) -> Result<usize, String> {
    let games = raw_history
        .get("games")
        .and_then(|value| value.get("games"))
        .and_then(Value::as_array)
        .ok_or_else(|| "match history response did not contain games.games".to_string())?;
    let connection = Connection::open(path)
        .map_err(|error| format!("failed opening agent database for games: {error}"))?;

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
                let Some(participant_id) = participant.get("participantId").and_then(Value::as_i64) else {
                    continue;
                };
                let stats = participant.get("stats");
                let item_ids: Vec<i64> = (0..=6)
                    .filter_map(|index| {
                        let key = format!("item{index}");
                        stats.and_then(|stats| stats.get(&key)).and_then(Value::as_i64)
                    })
                    .filter(|id| *id > 0)
                    .collect();
                let augment_ids = stats
                    .and_then(|stats| stats.get("augments"))
                    .cloned()
                    .unwrap_or_else(|| json!([]));
                transaction.execute(
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
                        stats.and_then(|stats| stats.get("win")).and_then(Value::as_bool).unwrap_or(false),
                        serde_json::to_string(&item_ids).unwrap_or_else(|_| "[]".to_string()),
                        serde_json::to_string(&augment_ids).unwrap_or_else(|_| "[]".to_string()),
                        stats.and_then(|stats| stats.get("kills")).and_then(Value::as_i64).unwrap_or(0),
                        stats.and_then(|stats| stats.get("deaths")).and_then(Value::as_i64).unwrap_or(0),
                        stats.and_then(|stats| stats.get("assists")).and_then(Value::as_i64).unwrap_or(0),
                        stats.and_then(|stats| stats.get("totalDamageDealtToChampions")).and_then(Value::as_i64),
                        stats.and_then(|stats| stats.get("goldEarned")).and_then(Value::as_i64).unwrap_or(0),
                    ],
                )
                .map_err(|error| format!("failed storing build facts for game {game_id}: {error}"))?;
            }
        }
    }
    transaction
        .commit()
        .map_err(|error| format!("failed committing games transaction: {error}"))?;
    Ok(games.len())
}

fn sync_state_value(path: &PathBuf, key: &str) -> Result<Option<String>, String> {
    let connection = Connection::open(path)
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
    let connection = Connection::open(path)
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
) {
    for game in page_games {
        let Some(game_id) = game.get("gameId").and_then(Value::as_i64) else {
            continue;
        };
        if shared_detail_cache_path(game_id).exists() {
            continue;
        }
        let route = format!("/lol-match-history/v1/games/{game_id}");
        {
            let mut state = runtime.write().await;
            state.collector_state = format!("backfilling-details:{discovered}");
        }
        match client.get_json::<Value>(&route).await {
            Ok(detail) => {
                if let Err(error) = cache_game_detail(game_id, &detail) {
                    logger::error("agent.collector", &format!("Failed caching detail for game {game_id}: {error}"));
                }
            }
            Err(error) => {
                logger::warn("agent.collector", &format!("Detail unavailable for game {game_id}: {error}"));
            }
        }
    }
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

    loop {
        let end = begin + PAGE_SIZE;
        let route = format!(
            "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex={begin}&endIndex={end}"
        );
        let history = client.get_json::<Value>(&route).await?;
        let page_games = history
            .get("games")
            .and_then(|value| value.get("games"))
            .and_then(Value::as_array)
            .ok_or_else(|| format!("match history page {begin}..{end} did not contain games.games"))?;

        if page_games.is_empty() {
            break;
        }

        let page_count = store_recent_games(path, &history, puuid)?;
        discovered += page_count;
        cache_page_details(client, page_games, runtime, discovered).await;
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
            break;
        }
        begin = end;

        // Prevent a malfunctioning client from producing an unbounded scan.
        if pages >= 1000 {
            logger::warn("agent.collector", &format!("Stopping history backfill after {pages} pages"));
            break;
        }
    }

    Ok(discovered)
}

async fn collector_loop(path: PathBuf, runtime: Arc<RwLock<RuntimeState>>) {
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
                logger::error("agent.collector", &format!("Failed creating LCU client: {error}"));
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
            state.collector_state = "collecting".to_string();
            state.league_client_connected = true;
        }

        let puuid = client
            .get_json::<Value>("/lol-summoner/v1/current-summoner")
            .await
            .ok()
            .and_then(|summoner| summoner.get("puuid").and_then(Value::as_str).map(str::to_string));
        let backfill_key = "match_history_backfill_puuid";
        let backfill_complete = sync_state_value(&path, backfill_key)
            .ok()
            .flatten()
            .as_deref()
            == puuid.as_deref();

        if !backfill_complete {
            match backfill_match_history(&client, &path, puuid.as_deref(), &runtime).await {
                Ok(count) => {
                    logger::info("agent.collector", &format!("History backfill complete: {count} records discovered"));
                    if let Some(puuid) = puuid.as_deref() {
                        if let Err(error) = set_sync_state_value(&path, backfill_key, puuid) {
                            logger::error("agent.collector", &format!("Failed recording history backfill state: {error}"));
                        }
                    }
                }
                Err(error) => logger::error("agent.collector", &format!("History backfill failed: {error}")),
            }
        } else {
            let route = "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=20";
            match client.get_json::<Value>(route).await {
                Ok(history) => match store_recent_games(&path, &history, puuid.as_deref()) {
                    Ok(count) => logger::debug("agent.collector", &format!("Reconciled {count} recent games")),
                    Err(error) => logger::error("agent.collector", &format!("Failed storing recent games: {error}")),
                },
                Err(error) => logger::error("agent.collector", &format!("Recent history reconciliation failed: {error}")),
            }
        }

        {
            let mut state = runtime.write().await;
            state.collector_state = "live".to_string();
        }
        sleep(Duration::from_secs(15)).await;
    }
}

async fn websocket_loop(runtime: Arc<RwLock<RuntimeState>>) {
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
                logger::error("agent.websocket", &format!("Failed creating TLS connector: {error}"));
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
