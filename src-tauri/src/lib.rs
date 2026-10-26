pub mod agent;
pub mod agent_supervisor;
pub mod commands;
pub mod logger;
pub mod models;
pub mod services;

use agent_supervisor::AgentSupervisor;
use commands::{
    agent::{
        api_agent_status, api_build_statistics, api_get_logs, api_global_aram_config,
        api_global_aram_statistics, api_log, api_pause_global_aram_crawler, api_reset_global_aram,
        api_set_global_aram_limit, api_set_global_aram_workers, api_statistics,
        api_stop_global_aram_crawler,
    },
    api_champ_select_session, api_delete_runeset, api_detect_champion_from_name,
    api_export_mayhem_build, api_export_saved_rune_page, api_get_champion_roster,
    api_get_champions, api_get_selected_champion_id, api_get_selected_rune_key,
    api_import_client_page, api_match_history, api_mayhem_augments, api_mayhem_champion_build,
    api_mayhem_team_builds, api_profile_challenge_tier, api_remove_friends,
    api_rune_builder_get_keystones, api_rune_builder_get_overview,
    api_rune_builder_get_secondary_playcounts, api_rune_builder_get_shards,
    api_rune_builder_get_tree_runes, api_rune_tree_client, api_rune_tree_current,
    api_rune_tree_saved, api_rune_tree_select_perk, api_runes_overview, api_runesets,
    api_save_rune_tree, api_set_selected_champion_id, api_set_selected_rune_key,
    api_summoner_profile, api_toggle_favorite_champion, api_update_runeset_champion,
};
use services::lcu_websocket::LcuWsWatcher;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            crate::logger::init_app_handle(app.handle().clone());
            let supervisor = AgentSupervisor::start()
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
            app.manage(supervisor);

            let status_item = MenuItem::with_id(app, "status", "Crawler: Idle", false, None::<&str>)?;
            let toggle_item = MenuItem::with_id(app, "toggle_window", "Hide LCU Companion", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&status_item, &sep1, &toggle_item, &sep2, &quit])?;

            let toggle_item_for_menu = toggle_item.clone();
            let toggle_item_for_tray = toggle_item.clone();
            let toggle_item_for_close = toggle_item.clone();

            TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "missing default application icon",
                    )
                })?)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "toggle_window" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                                let _ = toggle_item_for_menu.set_text("Show LCU Companion");
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = toggle_item_for_menu.set_text("Hide LCU Companion");
                            }
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                                let _ = toggle_item_for_tray.set_text("Show LCU Companion");
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = toggle_item_for_tray.set_text("Hide LCU Companion");
                            }
                        }
                    }
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let window_for_close = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_for_close.hide();
                        let _ = toggle_item_for_close.set_text("Show LCU Companion");
                    }
                });
            }

            // Spawn background task to update tray status item periodically
            let app_handle_for_status = app.handle().clone();
            let status_item_clone = status_item.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    if let Some(supervisor) = app_handle_for_status.try_state::<AgentSupervisor>() {
                        if let Ok(config) = supervisor
                            .request("global_aram_config")
                            .await
                        {
                            let paused = config
                                .get("crawler_paused")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let progress = config
                                .get("crawler_progress")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            let rate = config
                                .get("crawler_rate")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            let eta = config
                                .get("crawler_eta_text")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            let status_text = if paused {
                                "Crawler: Paused".to_string()
                            } else if rate > 0.05 || progress > 0.0 {
                                if !eta.is_empty() {
                                    format!("Crawler: {:.1}% ({})", progress, eta)
                                } else {
                                    format!("Crawler: {:.1}% ({:.1}/s)", progress, rate)
                                }
                            } else {
                                "Crawler: Idle".to_string()
                            };

                            let _ = status_item_clone.set_text(status_text);
                        }
                    }
                }
            });

            // Start the legacy Tauri watcher during the migration period.
            LcuWsWatcher::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api_agent_status,
            api_build_statistics,
            api_statistics,
            api_global_aram_config,
            api_global_aram_statistics,
            api_log,
            api_reset_global_aram,
            api_set_global_aram_limit,
            api_set_global_aram_workers,
            api_pause_global_aram_crawler,
            api_stop_global_aram_crawler,
            api_get_logs,
            api_summoner_profile,
            api_profile_challenge_tier,
            api_match_history,
            api_runes_overview,
            api_runesets,
            api_import_client_page,
            api_export_saved_rune_page,
            api_delete_runeset,
            api_update_runeset_champion,
            api_rune_tree_current,
            api_rune_tree_client,
            api_rune_tree_saved,
            api_rune_tree_select_perk,
            api_save_rune_tree,
            api_remove_friends,
            api_rune_builder_get_overview,
            api_rune_builder_get_keystones,
            api_rune_builder_get_tree_runes,
            api_rune_builder_get_secondary_playcounts,
            api_rune_builder_get_shards,
            api_get_champions,
            api_get_champion_roster,
            api_detect_champion_from_name,
            api_toggle_favorite_champion,
            api_get_selected_champion_id,
            api_set_selected_champion_id,
            api_get_selected_rune_key,
            api_set_selected_rune_key,
            api_champ_select_session,
            api_mayhem_champion_build,
            api_mayhem_team_builds,
            api_mayhem_augments,
            api_export_mayhem_build,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
