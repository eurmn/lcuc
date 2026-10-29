use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivePlayerItem {
    pub slot: u32,
    pub item_id: i64,
    pub count: u32,
    pub display_name: String,
    pub price: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LivePlayerStats {
    pub ability_power: f64,
    pub attack_damage: f64,
    pub attack_speed: f64,
    pub attack_range: f64,
    pub crit_chance: f64,
    pub armor: f64,
    pub magic_resist: f64,
    pub move_speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveGameState {
    pub is_in_game: bool,
    pub game_time: f64,
    pub champion_name: String,
    pub level: u32,
    pub current_gold: f64,
    pub items: Vec<LivePlayerItem>,
    pub stats: LivePlayerStats,
}

impl LiveGameState {
    pub fn not_in_game() -> Self {
        Self {
            is_in_game: false,
            game_time: 0.0,
            champion_name: String::new(),
            level: 0,
            current_gold: 0.0,
            items: Vec::new(),
            stats: LivePlayerStats::default(),
        }
    }
}

pub async fn fetch_live_game_state() -> Result<LiveGameState, String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_millis(1500))
        .build()
        .map_err(|e| format!("Failed to create client: {e}"))?;

    let resp = match client
        .get("https://127.0.0.1:2999/liveclientdata/allgamedata")
        .send()
        .await
    {
        Ok(res) => res,
        Err(_) => return Ok(LiveGameState::not_in_game()),
    };

    if !resp.status().is_success() {
        return Ok(LiveGameState::not_in_game());
    }

    let data: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return Ok(LiveGameState::not_in_game()),
    };

    let active_player = data.get("activePlayer");
    let current_gold = active_player
        .and_then(|p| p.get("currentGold"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let level = active_player
        .and_then(|p| p.get("level"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let summoner_name = active_player
        .and_then(|p| p.get("summonerName"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let riot_id = active_player
        .and_then(|p| p.get("riotId"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let stats_val = active_player.and_then(|p| p.get("championStats"));
    let stats = if let Some(s) = stats_val {
        LivePlayerStats {
            ability_power: s
                .get("abilityPower")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            attack_damage: s
                .get("attackDamage")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            attack_speed: s.get("attackSpeed").and_then(|v| v.as_f64()).unwrap_or(0.0),
            attack_range: s.get("attackRange").and_then(|v| v.as_f64()).unwrap_or(0.0),
            crit_chance: s.get("critChance").and_then(|v| v.as_f64()).unwrap_or(0.0),
            armor: s.get("armor").and_then(|v| v.as_f64()).unwrap_or(0.0),
            magic_resist: s.get("magicResist").and_then(|v| v.as_f64()).unwrap_or(0.0),
            move_speed: s.get("moveSpeed").and_then(|v| v.as_f64()).unwrap_or(0.0),
        }
    } else {
        LivePlayerStats::default()
    };

    let game_time = data
        .get("gameData")
        .and_then(|g| g.get("gameTime"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let mut items = Vec::new();
    let mut champ_name = String::new();

    if let Some(all_players) = data.get("allPlayers").and_then(|v| v.as_array()) {
        for player in all_players {
            let p_sum = player
                .get("summonerName")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let p_riot = player.get("riotId").and_then(|v| v.as_str()).unwrap_or("");
            let matched = (!summoner_name.is_empty() && p_sum == summoner_name)
                || (!riot_id.is_empty() && p_riot == riot_id)
                || all_players.len() == 1;

            if matched {
                champ_name = player
                    .get("championName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(raw_items) = player.get("items").and_then(|v| v.as_array()) {
                    for it in raw_items {
                        let item_id = it.get("itemID").and_then(|v| v.as_i64()).unwrap_or(0);
                        if item_id > 0 {
                            items.push(LivePlayerItem {
                                slot: it.get("slot").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                item_id,
                                count: it.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
                                display_name: it
                                    .get("displayName")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                price: it.get("price").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                            });
                        }
                    }
                }
                break;
            }
        }
    }

    Ok(LiveGameState {
        is_in_game: true,
        game_time,
        champion_name: champ_name,
        level,
        current_gold,
        items,
        stats,
    })
}

pub fn start_live_game_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut was_in_game = false;
        loop {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            match fetch_live_game_state().await {
                Ok(state) => {
                    if state.is_in_game {
                        was_in_game = true;
                        let _ = app.emit("live:game-state", &state);
                        let _ = app.emit("live:state-updated", &state);
                    } else if was_in_game {
                        was_in_game = false;
                        let not_in_game = LiveGameState::not_in_game();
                        let _ = app.emit("live:game-state", &not_in_game);
                        let _ = app.emit("live:state-updated", &not_in_game);
                    }
                }
                Err(_) => {
                    if was_in_game {
                        was_in_game = false;
                        let not_in_game = LiveGameState::not_in_game();
                        let _ = app.emit("live:game-state", &not_in_game);
                        let _ = app.emit("live:state-updated", &not_in_game);
                    }
                }
            }
        }
    });
}
