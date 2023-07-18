use tauri::{AppHandle, CustomMenuItem, Wry};

pub const QUIT_MENU_ID: &str = "quit";

#[derive(Clone)]
pub struct QuitActions {
    pub(crate) quit: CustomMenuItem,
}

impl QuitActions {
    pub fn new() -> QuitActions {
        let quit = CustomMenuItem::new("quit".to_string(), "Quit").accelerator("cmd+q");
        QuitActions { quit }
    }
}

/// Quit the application when the user wants to
pub fn on_quit(_app: &AppHandle<Wry>) -> tauri::Result<()> {
    std::process::exit(0);
}
