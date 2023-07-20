use crate::app::{AppState, TrayMenuItem, TrayMenuSection};
use crate::enroll::enroll_user::enroll_user;

use ockam_core::async_trait;
use tauri::{AppHandle, CustomMenuItem, Manager, SystemTrayMenu, Wry};

pub const ENROLL_MENU_HEADER_ID: &str = "enroll-header";
pub const ENROLL_MENU_ID: &str = "enroll";

pub struct EnrollTrayMenuSection {
    pub header: Option<TrayMenuItem>,
    pub enroll: Option<TrayMenuItem>,
}

impl EnrollTrayMenuSection {
    pub fn new() -> Self {
        Self {
            header: None,
            enroll: None,
        }
    }
}

impl Default for EnrollTrayMenuSection {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TrayMenuSection for EnrollTrayMenuSection {
    fn build(&self, tray_menu: SystemTrayMenu) -> SystemTrayMenu {
        let tray_menu = if let Some(header) = &self.header {
            tray_menu.add_item(header.inner())
        } else {
            tray_menu
        };
        if let Some(enroll) = &self.enroll {
            tray_menu.add_item(enroll.inner())
        } else {
            tray_menu
        }
    }

    async fn refresh(&mut self, app: &AppHandle<Wry>) {
        let state = app.state::<AppState>();
        if state.is_enrolled() {
            self.header = None;
            self.enroll = None;
        } else {
            self.header = Some(
                CustomMenuItem::new(ENROLL_MENU_HEADER_ID, "Please enroll")
                    .disabled()
                    .into(),
            );
            self.enroll = Some(
                CustomMenuItem::new(ENROLL_MENU_ID, "Enroll...")
                    .accelerator("cmd+e")
                    .into(),
            );
        }
    }
}

/// Event listener for the "Enroll" menu item
/// Enroll the user and show that it has been enrolled
pub fn on_enroll(app: &AppHandle<Wry>) -> tauri::Result<()> {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move { enroll_user(&app_handle).await });
    Ok(())
}
