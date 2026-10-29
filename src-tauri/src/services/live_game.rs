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
    pub game_mode: String,
    pub game_time: f64,
    pub champion_name: String,
    pub level: u32,
    pub current_gold: f64,
    pub is_dead: bool,
    pub respawn_timer: f64,
    pub items: Vec<LivePlayerItem>,
    pub stats: LivePlayerStats,
}

impl LiveGameState {
    pub fn not_in_game() -> Self {
        Self {
            is_in_game: false,
            game_mode: String::new(),
            game_time: 0.0,
            champion_name: String::new(),
            level: 0,
            current_gold: 0.0,
            is_dead: false,
            respawn_timer: 0.0,
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
    let game_mode = data
        .get("gameData")
        .and_then(|g| g.get("gameMode"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut items = Vec::new();
    let mut champ_name = String::new();
    let mut is_dead = false;
    let mut respawn_timer = 0.0;

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
                is_dead = player
                    .get("isDead")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                respawn_timer = player
                    .get("respawnTimer")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
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
        game_mode,
        game_time,
        champion_name: champ_name,
        level,
        current_gold,
        is_dead,
        respawn_timer,
        items,
        stats,
    })
}

pub fn start_live_game_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut was_in_game = false;
        let mut armed_milestone: Option<u32> = None;
        let mut completed_milestones: u8 = 0;
        let mut was_dead = false;
        let mut cards_visible = false;
        let mut scan_attempts: u32 = 0;
        let mut last_scan_time = std::time::Instant::now();
        let mut last_card_ids: Vec<String> = Vec::new();

        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let window_rect = crate::services::live_overlay::get_league_game_window_rect();
            let _ = app.emit("live:window-rect", &window_rect);

            match fetch_live_game_state().await {
                Ok(state) => {
                    if state.is_in_game {
                        was_in_game = true;
                        let _ = app.emit("live:game-state", &state);
                        let _ = app.emit("live:state-updated", &state);

                        let overlay_enabled = crate::services::live_overlay::is_overlay_enabled();
                        if overlay_enabled {
                            // Resolve champion ID by name
                            let champion_id = if !state.champion_name.is_empty() {
                                crate::services::champions::get_all_champions()
                                    .await
                                    .ok()
                                    .and_then(|champs| {
                                        champs
                                            .into_iter()
                                            .find(|c| {
                                                c.name.eq_ignore_ascii_case(&state.champion_name)
                                                    || c.alias
                                                        .eq_ignore_ascii_case(&state.champion_name)
                                            })
                                            .map(|c| c.id)
                                    })
                            } else {
                                None
                            };

                            let current_augments = Vec::<String>::new();

                            // Arming logic
                            if state.level >= 3 && (completed_milestones & 1 == 0) {
                                if state.game_time < 95.0 || !state.is_dead {
                                    armed_milestone = Some(3);
                                }
                            }
                            if (7..11).contains(&state.level) && (completed_milestones & 2 == 0) {
                                if (was_dead && !state.is_dead) || state.game_time < 90.0 {
                                    armed_milestone = Some(7);
                                }
                            }
                            if (11..15).contains(&state.level) && (completed_milestones & 4 == 0) {
                                if was_dead && !state.is_dead {
                                    armed_milestone = Some(11);
                                }
                            }
                            if state.level >= 15 && (completed_milestones & 8 == 0) {
                                if was_dead && !state.is_dead {
                                    armed_milestone = Some(15);
                                }
                            }

                            // Active card display logic
                            let now = std::time::Instant::now();
                            if cards_visible {
                                if now.duration_since(last_scan_time) >= Duration::from_millis(600)
                                {
                                    last_scan_time = now;
                                    let is_foreground = crate::services::live_overlay::get_league_window_rect()
                                        .map(|r| r.is_foreground)
                                        .unwrap_or(false);

                                    let cards = if is_foreground {
                                        crate::services::live_overlay::scan_in_game_augment_cards(
                                            champion_id,
                                            current_augments.clone(),
                                        )
                                        .await
                                        .unwrap_or_default()
                                    } else {
                                        Vec::new()
                                    };

                                    if cards.len() < 2 || !is_foreground {
                                        crate::services::live_overlay::hide_overlay_window(&app);
                                        let _ = app.emit(
                                            "overlay:cards-updated",
                                            &Vec::<crate::models::history::OverlayAugmentCard>::new(),
                                        );
                                        cards_visible = false;
                                        if let Some(m) = armed_milestone {
                                            let bit = match m {
                                                3 => 1,
                                                7 => 2,
                                                11 => 4,
                                                15 => 8,
                                                _ => 0,
                                            };
                                            completed_milestones |= bit;
                                            armed_milestone = None;
                                        }
                                    } else {
                                        let new_ids: Vec<String> =
                                            cards.iter().map(|c| c.id.clone()).collect();
                                        if new_ids != last_card_ids {
                                            last_card_ids = new_ids;
                                            let _ = app.emit("overlay:cards-updated", &cards);
                                        }
                                    }
                                }
                            } else if armed_milestone.is_some() {
                                let interval = if scan_attempts < 25 {
                                    Duration::from_millis(750)
                                } else {
                                    Duration::from_millis(2500)
                                };

                                if now.duration_since(last_scan_time) >= interval {
                                    last_scan_time = now;
                                    scan_attempts += 1;
                                    let cards =
                                        crate::services::live_overlay::scan_in_game_augment_cards(
                                            champion_id,
                                            current_augments,
                                        )
                                        .await
                                        .unwrap_or_default();

                                    if cards.len() >= 2 {
                                        cards_visible = true;
                                        scan_attempts = 0;
                                        last_card_ids =
                                            cards.iter().map(|c| c.id.clone()).collect();
                                        if let Some(rect) =
                                            crate::services::live_overlay::get_league_window_rect()
                                        {
                                            crate::services::live_overlay::show_overlay_banner_rect(
                                                &app, &rect,
                                            );
                                        }
                                        let _ = app.emit("overlay:cards-updated", &cards);
                                    }
                                }
                            }
                        }

                        was_dead = state.is_dead;
                    } else if was_in_game {
                        was_in_game = false;
                        completed_milestones = 0;
                        armed_milestone = None;
                        cards_visible = false;
                        crate::services::live_overlay::hide_overlay_window(&app);
                        let not_in_game = LiveGameState::not_in_game();
                        let _ = app.emit("live:game-state", &not_in_game);
                        let _ = app.emit("live:state-updated", &not_in_game);
                    }
                }
                Err(_) => {
                    if was_in_game {
                        was_in_game = false;
                        completed_milestones = 0;
                        armed_milestone = None;
                        cards_visible = false;
                        crate::services::live_overlay::hide_overlay_window(&app);
                        let not_in_game = LiveGameState::not_in_game();
                        let _ = app.emit("live:game-state", &not_in_game);
                        let _ = app.emit("live:state-updated", &not_in_game);
                    }
                }
            }
        }
    });
}
