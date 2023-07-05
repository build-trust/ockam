use tauri::{AppHandle, CustomMenuItem, Wry};

pub const QUIT_MENU_ID: &str = "quit";

pub fn menu_items() -> Vec<CustomMenuItem> {
    vec![CustomMenuItem::new("quit".to_string(), "Quit").accelerator("cmd+q")]
}

/// Quit the application when the user wants to
pub fn on_quit(_app: &AppHandle<Wry>) -> tauri::Result<()> {
    std::process::exit(0);
}
