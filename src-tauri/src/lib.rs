pub mod commands;
pub mod models;
pub mod services;

use commands::{
    api_delete_runeset, api_detect_champion_from_name, api_export_saved_rune_page,
    api_get_champion_roster, api_get_champions, api_get_selected_champion_id,
    api_get_selected_rune_key, api_import_client_page, api_profile_challenge_tier,
    api_remove_friends, api_rune_builder_get_keystones, api_rune_builder_get_overview,
    api_rune_builder_get_secondary_playcounts, api_rune_builder_get_shards,
    api_rune_builder_get_tree_runes, api_rune_tree_client, api_rune_tree_current,
    api_rune_tree_saved, api_rune_tree_select_perk, api_runes_overview, api_runesets,
    api_save_rune_tree, api_set_selected_champion_id, api_set_selected_rune_key,
    api_summoner_profile, api_toggle_favorite_champion, api_update_runeset_champion,
};
use services::lcu_websocket::LcuWsWatcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Start background LCU WebSocket listener and companion slot watcher
            LcuWsWatcher::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api_summoner_profile,
            api_profile_challenge_tier,
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

