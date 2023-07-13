use crate::app::tray_menu::{TrayMenuItem, TrayMenuSection};
use crate::enroll::Backend;
use crate::AppHandle;
use ockam_api::cli_state::StateDirTrait;
use ockam_command::{CommandGlobalOpts, GlobalArgs};
use tauri::{CustomMenuItem, SystemTrayMenu};

pub const RESET_MENU_ID: &str = "reset";
pub const QUIT_MENU_ID: &str = "quit";

pub struct OptionsTrayMenuSection {
    pub reset: TrayMenuItem,
    pub quit: TrayMenuItem,
}

impl OptionsTrayMenuSection {
    pub fn new() -> Self {
        let opts = CommandGlobalOpts::new(GlobalArgs::default());
        let reset_disabled = opts.state.projects.default().is_err();
        let mut reset = CustomMenuItem::new(RESET_MENU_ID, "Reset...").accelerator("cmd+r");
        if reset_disabled {
            reset = reset.disabled();
        }
        Self {
            reset: reset.into(),
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

impl TrayMenuSection for OptionsTrayMenuSection {
    fn build(&self, tray_menu: SystemTrayMenu) -> SystemTrayMenu {
        tray_menu
            .add_item(self.reset.inner())
            .add_item(self.quit.inner())
    }
}

/// Event listener for the "Reset" menu item
/// Reset the persistent state
pub fn on_reset(backend: impl Backend, app_handle: AppHandle) -> tauri::Result<()> {
    let _ = backend.reset(app_handle);
    Ok(())
}

/// Event listener for the "Quit" menu item
/// Quit the application when the user wants to
pub fn on_quit() -> tauri::Result<()> {
    std::process::exit(0);
}
