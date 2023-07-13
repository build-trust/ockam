use crate::app::tray_menu::{TrayMenuItem, TrayMenuSection};
use crate::enroll::Backend;
use crate::AppHandle;
use ockam_api::cli_state::StateDirTrait;
use ockam_command::{CommandGlobalOpts, GlobalArgs};
use tauri::{CustomMenuItem, SystemTrayMenu};

pub const ENROLL_MENU_ID: &str = "enroll";

pub struct EnrollTrayMenuSection {
    pub enroll: TrayMenuItem,
}

impl EnrollTrayMenuSection {
    pub fn new() -> Self {
        let opts = CommandGlobalOpts::new(GlobalArgs::default());
        let disabled = opts.state.projects.default().is_ok();
        let mut item = CustomMenuItem::new(ENROLL_MENU_ID, "Enroll...").accelerator("cmd+e");
        if disabled {
            item = item.disabled();
        }
        Self {
            enroll: item.into(),
        }
    }
}

impl Default for EnrollTrayMenuSection {
    fn default() -> Self {
        Self::new()
    }
}

impl TrayMenuSection for EnrollTrayMenuSection {
    fn build(&self, tray_menu: SystemTrayMenu) -> SystemTrayMenu {
        tray_menu.add_item(self.enroll.inner())
    }
}

/// Event listener for the "Enroll" menu item
/// Enroll the user and show that it has been enrolled
pub fn on_enroll(backend: impl Backend, app_handle: AppHandle) -> tauri::Result<()> {
    let _ = backend.enroll_user(app_handle);
    Ok(())
}
