pub mod agent;
pub mod agent_supervisor;
pub mod commands;
pub mod logger;
pub mod models;
pub mod services;

use agent_supervisor::AgentSupervisor;
use commands::{
    agent::{
        api_agent_status, api_build_statistics, api_global_aram_config, api_global_aram_statistics,
        api_log, api_set_global_aram_limit, api_statistics,
    },
    api_delete_runeset, api_detect_champion_from_name, api_export_saved_rune_page,
    api_get_champion_roster, api_get_champions, api_get_selected_champion_id,
    api_get_selected_rune_key, api_import_client_page, api_match_history,
    api_profile_challenge_tier, api_remove_friends, api_rune_builder_get_keystones,
    api_rune_builder_get_overview, api_rune_builder_get_secondary_playcounts,
    api_rune_builder_get_shards, api_rune_builder_get_tree_runes, api_rune_tree_client,
    api_rune_tree_current, api_rune_tree_saved, api_rune_tree_select_perk, api_runes_overview,
    api_runesets, api_save_rune_tree, api_set_selected_champion_id, api_set_selected_rune_key,
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
            let supervisor = AgentSupervisor::start()
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
            app.manage(supervisor);
            let show = MenuItem::with_id(app, "show", "Show LCU Companion", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&show, &separator, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "missing default application icon",
                    )
                })?)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
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
                    }
                });
            }

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
            api_set_global_aram_limit,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
