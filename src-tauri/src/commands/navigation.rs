use crate::render::render_shell;

#[tauri::command]
pub fn ui_shell(tab: Option<String>) -> String {
    let tab_name = tab.as_deref().unwrap_or("runes");
    render_shell(tab_name)
}
