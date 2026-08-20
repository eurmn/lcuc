pub mod commands;
pub mod models;
pub mod render;
pub mod services;

use commands::{
    api_remove_friends, api_runesets, api_summoner_profile, ui_delete_runeset,
    ui_export_saved_rune_page, ui_import_client_page, ui_import_client_page_from_tree,
    ui_profile_card, ui_profile_challenge_tier, ui_rune_tree_cancel_edit, ui_rune_tree_client,
    ui_rune_tree_current, ui_rune_tree_edit_mode, ui_rune_tree_saved, ui_rune_tree_select_perk,
    ui_runesets, ui_save_rune_tree, ui_shell,
};
use services::lcu_websocket::LcuWsWatcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            // Start background LCU WebSocket listener and companion slot watcher
            LcuWsWatcher::start();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ui_shell,
            ui_profile_card,
            ui_profile_challenge_tier,
            ui_runesets,
            ui_import_client_page,
            ui_import_client_page_from_tree,
            ui_export_saved_rune_page,
            ui_delete_runeset,
            ui_rune_tree_current,
            ui_rune_tree_client,
            ui_rune_tree_saved,
            ui_rune_tree_edit_mode,
            ui_rune_tree_cancel_edit,
            ui_rune_tree_select_perk,
            ui_save_rune_tree,
            api_summoner_profile,
            api_runesets,
            api_remove_friends,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
