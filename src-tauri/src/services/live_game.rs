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
    pub is_augment_checkpoint: bool,
    pub augment_slot: Option<u8>,

    // camelCase mirrors
    pub isInGame: bool,
    pub gameMode: String,
    pub gameTime: f64,
    pub championName: String,
    pub currentGold: f64,
    pub isDead: bool,
    pub respawnTimer: f64,
    pub isAugmentCheckpoint: bool,
    pub augmentSlot: Option<u8>,
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
            is_augment_checkpoint: false,
            augment_slot: None,
            isInGame: false,
            gameMode: String::new(),
            gameTime: 0.0,
            championName: String::new(),
            currentGold: 0.0,
            isDead: false,
            respawnTimer: 0.0,
            isAugmentCheckpoint: false,
            augmentSlot: None,
        }
    }
}

pub async fn fetch_live_game_state() -> Result<LiveGameState, String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_millis(800))
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
                        // Filter out empty slots, Poro-Snax (2050, 2052, 220013)
                        if item_id > 0 && item_id != 2050 && item_id != 2052 && item_id != 220013 {
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

    let is_augment_checkpoint = (level > 0 && level <= 3) || level == 7 || level == 11 || level == 15;
    let augment_slot = match level {
        1..=3 => Some(1),
        7 => Some(2),
        11 => Some(3),
        15 => Some(4),
        _ => None,
    };

    Ok(LiveGameState {
        is_in_game: true,
        game_mode: game_mode.clone(),
        game_time,
        champion_name: champ_name.clone(),
        level,
        current_gold,
        is_dead,
        respawn_timer,
        items,
        stats,
        is_augment_checkpoint,
        augment_slot,
        isInGame: true,
        gameMode: game_mode,
        gameTime: game_time,
        championName: champ_name,
        currentGold: current_gold,
        isDead: is_dead,
        respawnTimer: respawn_timer,
        isAugmentCheckpoint: is_augment_checkpoint,
        augmentSlot: augment_slot,
    })
}

/// One F8 press, end to end. Runs off the polling loop so a slow scan can never
/// stall the HUD.
pub async fn run_f8_scan(app: AppHandle, champion_name: String) {
    run_scan(app, champion_name, false).await
}

/// `auto` scans are triggered by the panel watcher: they must fail silently,
/// because a false positive during a fight is not the player's problem.
pub async fn run_scan(app: AppHandle, champion_name: String, auto: bool) {
    if crate::services::live_overlay::scan_in_flight() {
        return;
    }
    crate::services::live_overlay::set_overlay_scanning(true);
    let _ = app.emit("overlay:scan-status", &true);

    let champion_id = if champion_name.is_empty() {
        None
    } else {
        crate::services::champions::get_all_champions()
            .await
            .ok()
            .and_then(|champs| {
                champs
                    .into_iter()
                    .find(|c| {
                        c.name.eq_ignore_ascii_case(&champion_name)
                            || c.alias.eq_ignore_ascii_case(&champion_name)
                    })
                    .map(|c| c.id)
            })
    };

    let started = std::time::Instant::now();
    let scan_res =
        crate::services::live_overlay::scan_in_game_augment_cards(champion_id, Vec::new()).await;
    let elapsed = started.elapsed().as_millis();

    crate::services::live_overlay::set_overlay_scanning(false);
    let _ = app.emit("overlay:scan-status", &false);

    match scan_res {
        Ok(cards) if !cards.is_empty() => {
            crate::logger::write(
                crate::logger::Level::Info,
                "live_game",
                &format!(
                    "F8 scan identified {} cards in {elapsed}ms: {}",
                    cards.len(),
                    cards
                        .iter()
                        .map(|c| format!("{}={}", c.slot, c.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                None,
            );
            crate::services::live_overlay::update_overlay_cards(&cards);
            let rect = crate::services::live_overlay::get_league_game_window_rect();
            crate::services::live_overlay::show_overlay_banner_rect(&app, &rect);
            let _ = app.emit("overlay:cards-updated", &cards);
        }
        Ok(_) => {
            crate::services::live_overlay::clear_overlay_cards();
            if !auto {
                crate::services::live_overlay::show_overlay_status_toast("No cards detected", true);
            }
            let _ = app.emit(
                "overlay:cards-updated",
                &Vec::<crate::models::history::OverlayAugmentCard>::new(),
            );
        }
        Err(err_msg) => {
            crate::logger::write(
                crate::logger::Level::Warn,
                "live_game",
                &format!("F8 scan failed after {elapsed}ms: {err_msg}"),
                None,
            );
            crate::services::live_overlay::clear_overlay_cards();
            if !auto {
                crate::services::live_overlay::show_overlay_status_toast(&err_msg, true);
            }
            let _ = app.emit(
                "overlay:cards-updated",
                &Vec::<crate::models::history::OverlayAugmentCard>::new(),
            );
        }
    }
}

/// Manual pick from an overlay hotkey: records the card in that slot and says so.
/// Returns true when something was actually recorded.
fn record_pick(app: &AppHandle, slot: usize) -> bool {
    match crate::services::live_overlay::record_pick_for_slot(slot) {
        Some(owned) => {
            let _ = app.emit("overlay:augment-picked", &owned);
            true
        }
        None => {
            crate::services::live_overlay::show_overlay_status_toast(
                "No cards on screen — press F8 first",
                true,
            );
            false
        }
    }
}

/// Pick detected by watching the panel: silent about the pick itself (the caller
/// toasts), but returns what was recorded.
fn record_pick_silent(slot: usize) -> Option<crate::services::live_overlay::OwnedAugment> {
    crate::services::live_overlay::record_pick_for_slot(slot)
}

/// Watches the augment panel so the player never has to press F8 mid-fight, and
/// remembers which card was highlighted when the panel closes.
#[derive(Default)]
struct PanelWatcher {
    /// Cards were on screen at the previous probe.
    visible: bool,
    /// The panel currently up has already been scanned.
    scanned: bool,
    /// Best highlight seen for this appearance, and which slot.
    best: (usize, f32),
    /// Per-card brightness when the panel first settled. Picks are judged against
    /// this, not against the other cards: the middle card is always brighter.
    baseline: [f32; 3],
    /// Consecutive probes that saw the highlight, to reject one-frame noise.
    votes: u8,
    /// Art signature of the cards we last looked at, for reroll detection.
    signature: Vec<f32>,
    /// The player already recorded a pick for this appearance (hotkey or auto), so
    /// the panel closing must not ask again.
    picked: bool,
}

impl PanelWatcher {
    fn reset(&mut self) {
        self.visible = false;
        self.scanned = false;
        self.best = (0, 0.0);
        self.votes = 0;
        self.baseline = [0.0; 3];
        self.signature.clear();
        self.picked = false;
    }
}

pub fn start_live_game_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut was_in_game = false;
        let mut cards_visible = false;
        let mut tick: u64 = 0;
        let mut last_champ_name = String::new();
        let mut watcher = PanelWatcher::default();
        let mut at_checkpoint = false;

        loop {
            let window_rect = crate::services::live_overlay::get_league_game_window_rect();

            if !window_rect.found {
                if was_in_game {
                    was_in_game = false;
                    cards_visible = false;
                    at_checkpoint = false;
                    watcher.reset();
                    last_champ_name.clear();
                    crate::services::live_overlay::reset_owned_augments();
                    crate::services::live_overlay::reset_inventory_fingerprint();
                    crate::services::live_overlay::clear_overlay_cards();
                    crate::services::live_overlay::hide_overlay_window(&app);
                    let not_in_game = LiveGameState::not_in_game();
                    let _ = app.emit("live:game-state", &not_in_game);
                    let _ = app.emit("live:state-updated", &not_in_game);
                }
                tokio::time::sleep(Duration::from_millis(1500)).await;
                continue;
            }

            tokio::time::sleep(Duration::from_millis(75)).await;
            cards_visible = crate::services::live_overlay::overlay_has_cards();

            if tick % 20 == 0 {
                let _ = app.emit("live:window-rect", &window_rect);
                match fetch_live_game_state().await {
                    Ok(state) => {
                        if state.is_in_game {
                            if !was_in_game {
                                crate::logger::write(
                                    crate::logger::Level::Info,
                                    "live_game",
                                    &format!("Detected active match with champion '{}', window at ({},{}) {}x{}, foreground={}", state.champion_name, window_rect.x, window_rect.y, window_rect.width, window_rect.height, window_rect.is_foreground),
                                    None,
                                );
                            }
                            let entering = !was_in_game;
                            was_in_game = true;
                            last_champ_name = state.champion_name.clone();
                            at_checkpoint = state.is_augment_checkpoint;
                            crate::services::live_overlay::set_overlay_champion_and_strategy(&state.champion_name, "");
                            crate::services::live_overlay::set_overlay_vitals(state.level, state.current_gold);
                            let _ = app.emit("live:game-state", &state);
                            let _ = app.emit("live:state-updated", &state);

                            if entering {
                                // The build panel needs the champion id, and the
                                // scanner only records it once a card is scanned.
                                let name = state.champion_name.clone();
                                let champ_id = crate::services::champions::get_all_champions()
                                    .await
                                    .ok()
                                    .and_then(|champs| {
                                        champs
                                            .into_iter()
                                            .find(|c| {
                                                c.name.eq_ignore_ascii_case(&name)
                                                    || c.alias.eq_ignore_ascii_case(&name)
                                            })
                                            .map(|c| c.id as i64)
                                    });
                                crate::services::live_overlay::remember_champion_id(champ_id);
                            }

                            // Buying or selling an item re-ranks every remaining
                            // slot, so the three rows must follow the inventory.
                            let item_ids: Vec<i64> =
                                state.items.iter().map(|i| i.item_id).collect();
                            let fingerprint =
                                crate::services::live_overlay::inventory_fingerprint(&item_ids);
                            if crate::services::live_overlay::note_inventory_fingerprint(fingerprint) {
                                crate::services::live_overlay::refresh_in_game_build();
                            }
                        } else if was_in_game {
                            crate::logger::write(
                                crate::logger::Level::Info,
                                "live_game",
                                "Match ended or player exited game",
                                None,
                            );
                            was_in_game = false;
                            cards_visible = false;
                            at_checkpoint = false;
                            watcher.reset();
                            last_champ_name.clear();
                            crate::services::live_overlay::reset_owned_augments();
                            crate::services::live_overlay::reset_inventory_fingerprint();
                            crate::services::live_overlay::clear_overlay_cards();
                            crate::services::live_overlay::hide_overlay_window(&app);
                            let not_in_game = LiveGameState::not_in_game();
                            let _ = app.emit("live:game-state", &not_in_game);
                            let _ = app.emit("live:state-updated", &not_in_game);
                        }
                    }
                    Err(err) => {
                        if was_in_game {
                            crate::logger::write(
                                crate::logger::Level::Warn,
                                "live_game",
                                &format!("Failed to poll Live Client API while in game: {err}"),
                                None,
                            );
                            was_in_game = false;
                            cards_visible = false;
                            at_checkpoint = false;
                            watcher.reset();
                            last_champ_name.clear();
                            crate::services::live_overlay::reset_owned_augments();
                            crate::services::live_overlay::reset_inventory_fingerprint();
                            crate::services::live_overlay::clear_overlay_cards();
                            crate::services::live_overlay::hide_overlay_window(&app);
                            let not_in_game = LiveGameState::not_in_game();
                            let _ = app.emit("live:game-state", &not_in_game);
                            let _ = app.emit("live:state-updated", &not_in_game);
                        }
                    }
                }
            }

            let overlay_enabled = crate::services::live_overlay::is_overlay_enabled();
            if !overlay_enabled || !window_rect.found || !window_rect.is_foreground {
                if tick % 40 == 0 && was_in_game {
                    crate::logger::write(
                        crate::logger::Level::Debug,
                        "live_game",
                        &format!("Overlay suppressed: enabled={overlay_enabled}, window_found={}, is_foreground={}", window_rect.found, window_rect.is_foreground),
                        None,
                    );
                }
                crate::services::live_overlay::hide_overlay_window(&app);
                tick = tick.wrapping_add(1);
                continue;
            }

            if tick % 20 == 0 && (was_in_game || cards_visible) {
                crate::services::live_overlay::show_overlay_banner_rect(&app, &window_rect);
            }

            let keys = crate::services::live_overlay::poll_overlay_hotkeys();
            if keys.scan {
                // Never await the scan here: this loop also drives the HUD and
                // the augment-checkpoint detection, and a slow scan used to
                // stall all of it.
                let app_handle = app.clone();
                let champ_name = last_champ_name.clone();
                watcher.scanned = true;
                tauri::async_runtime::spawn(async move {
                    run_f8_scan(app_handle, champ_name).await;
                });
            } else if keys.clear && cards_visible {
                crate::services::live_overlay::clear_overlay_cards();
                let _ = app.emit(
                    "overlay:cards-updated",
                    &Vec::<crate::models::history::OverlayAugmentCard>::new(),
                );
                cards_visible = false;
                watcher.reset();
            }

            if keys.undo {
                match crate::services::live_overlay::undo_owned_augment() {
                    Some(removed) => {
                        crate::services::live_overlay::show_overlay_status_toast(
                            &format!("Removed {}", removed.name),
                            false,
                        );
                    }
                    None => {}
                }
            }
            if let Some(slot) = keys.picked_slot() {
                if record_pick(&app, slot) {
                    watcher.picked = true;
                }
            }

            // Auto-detect the augment panel. Probing is a plain screen capture
            // plus a brightness scan, so it is only worth doing near a
            // checkpoint; elsewhere it drops to once every ~1.5s.
            let probe_every = if at_checkpoint || cards_visible { 4 } else { 20 };
            #[cfg(windows)]
            if tick % probe_every == 0 && !crate::services::live_overlay::scan_in_flight() {
                match crate::services::live_overlay::probe_augment_panel() {
                    Some(probe) => {
                        if !watcher.visible {
                            watcher.visible = true;
                            watcher.scanned = false;
                            watcher.best = (0, 0.0);
                            watcher.votes = 0;
                            watcher.baseline = probe.selection;
                            watcher.signature = probe.signature.clone();
                        } else if crate::services::augment_scan::signature_changed(
                            &watcher.signature,
                            &probe.signature,
                        ) {
                            // The cards changed without the panel closing: a
                            // reroll. Drop the stale scan, baseline and pick votes.
                            watcher.signature = probe.signature.clone();
                            watcher.baseline = probe.selection;
                            watcher.scanned = false;
                            watcher.votes = 0;
                            watcher.best = (0, 0.0);
                        }
                        // Which card lit up relative to how it looked when the
                        // panel appeared. Only trust a repeatable signal.
                        if let Some(slot) = crate::services::augment_scan::picked_from_baseline(
                            watcher.baseline,
                            probe.selection,
                        )
                        {
                            watcher.votes = watcher.votes.saturating_add(1);
                            watcher.best = (slot, probe.selection[slot]);
                        } else {
                            watcher.votes = 0;
                        }

                        if !watcher.scanned {
                            watcher.scanned = true;
                            cards_visible = true;
                            let app_handle = app.clone();
                            let champ_name = last_champ_name.clone();
                            crate::logger::write(
                                crate::logger::Level::Info,
                                "live_game",
                                "augment panel detected, scanning automatically",
                                None,
                            );
                            tauri::async_runtime::spawn(async move {
                                run_scan(app_handle, champ_name, true).await;
                            });
                        }
                    }
                    None => {
                        if watcher.visible {
                            // Panel closed: if one card was clearly lit up just
                            // before it went away, that was the pick.
                            let suspect = (watcher.votes >= 2).then_some(watcher.best.0);
                            let already_picked = watcher.picked;
                            watcher.reset();
                            cards_visible = false;
                            match suspect.and_then(record_pick_silent) {
                                Some(owned) => {
                                    crate::services::live_overlay::show_overlay_status_toast(
                                        &format!("Picked {}", owned.name),
                                        false,
                                    );
                                    crate::services::live_overlay::clear_overlay_cards();
                                    let _ = app.emit(
                                        "overlay:cards-updated",
                                        &Vec::<crate::models::history::OverlayAugmentCard>::new(),
                                    );
                                }
                                None => {
                                    crate::services::live_overlay::clear_overlay_cards();
                                    let _ = app.emit(
                                        "overlay:cards-updated",
                                        &Vec::<crate::models::history::OverlayAugmentCard>::new(),
                                    );
                                    // Auto-detection is deliberately conservative;
                                    // say which keys record it by hand.
                                    if !already_picked {
                                        crate::services::live_overlay::show_overlay_status_toast(
                                            "Picked which card? F9 / F10 / F11",
                                            false,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            tick = tick.wrapping_add(1);
        }
    });
}
