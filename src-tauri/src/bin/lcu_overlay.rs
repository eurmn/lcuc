#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    lcu_companion_lib::services::native_overlay::run_detached_overlay_process();
}
