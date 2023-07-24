use tauri::{AppHandle, CustomMenuItem, SystemTrayMenu, Wry};

use ockam_core::async_trait;

use crate::app::{AppState, TrayMenuItem, TrayMenuSection};
use crate::options::reset;

pub const RESET_MENU_ID: &str = "reset";
pub const QUIT_MENU_ID: &str = "quit";

pub struct OptionsTrayMenuSection {
    pub reset: Option<TrayMenuItem>,
    pub quit: TrayMenuItem,
}

impl OptionsTrayMenuSection {
    pub fn new() -> Self {
        Self {
            reset: None,
            quit: CustomMenuItem::new(QUIT_MENU_ID, "Quit")
                .accelerator("cmd+q")
                .into(),
        }
    }
}

impl Default for OptionsTrayMenuSection {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TrayMenuSection for OptionsTrayMenuSection {
    fn build(&self, tray_menu: SystemTrayMenu) -> SystemTrayMenu {
        let tray_menu = if let Some(reset) = &self.reset {
            tray_menu.add_item(reset.inner())
        } else {
            tray_menu
        };
        tray_menu.add_item(self.quit.inner())
    }

    async fn refresh(&mut self, app_state: &AppState) {
        if app_state.is_enrolled() {
            self.reset = Some(
                CustomMenuItem::new(RESET_MENU_ID, "Reset")
                    .accelerator("cmd+r")
                    .into(),
            );
        } else {
            self.reset = None;
        }
    }
}

/// Event listener for the "Reset" menu item
/// Reset the persistent state
pub fn on_reset(app: &AppHandle<Wry>) -> tauri::Result<()> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { reset(&app).await });
    Ok(())
}

/// Event listener for the "Quit" menu item
/// Quit the application when the user wants to
pub fn on_quit() -> tauri::Result<()> {
    std::process::exit(0);
}
