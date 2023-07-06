use tauri::{AppHandle, CustomMenuItem, Wry};
use tracing::info;

use ockam_api::cli_state::StateDirTrait;
use ockam_command::{CommandGlobalOpts, GlobalArgs};

use crate::enroll::backend::Backend;

pub const ENROLL_MENU_ID: &str = "enroll";
pub const RESET_MENU_ID: &str = "reset";

pub fn menu_items() -> Vec<CustomMenuItem> {
    let options = CommandGlobalOpts::new(GlobalArgs::default());

    let enroll_menu_item = CustomMenuItem::new(ENROLL_MENU_ID, "Enroll...").accelerator("cmd+e");
    let reset_menu_item = CustomMenuItem::new(RESET_MENU_ID, "Reset...").accelerator("cmd+r");
    match options.state.projects.default() {
        Ok(_) => vec![enroll_menu_item.disabled(), reset_menu_item],
        Err(_) => {
            info!("There is no default project, please enroll");
            vec![enroll_menu_item, reset_menu_item.disabled()]
        }
    }
}

/// Enroll the user and show that it has been enrolled
pub fn on_enroll(backend: impl Backend, app: &AppHandle<Wry>) -> tauri::Result<()> {
    if backend.enroll_user().is_ok() {
        app.tray_handle()
            .get_item(ENROLL_MENU_ID)
            .set_enabled(false)?;
        app.tray_handle().get_item(RESET_MENU_ID).set_enabled(true)
    } else {
        Ok(())
    }
}

/// Reset the persistent state
pub fn on_reset(backend: impl Backend, app: &AppHandle<Wry>) -> tauri::Result<()> {
    if backend.reset().is_ok() {
        app.tray_handle()
            .get_item(ENROLL_MENU_ID)
            .set_enabled(true)?;
        app.tray_handle().get_item(RESET_MENU_ID).set_enabled(false)
    } else {
        Ok(())
    }
}

#[cfg(tests)]
mod tests {
    use super::*;

    fn test_enroll_reset() {
        let backend = TestBackend {};
        let app = tauri::Builder::default()
            .build(tauri::generate_context!())
            .unwrap();
        on_enroll(backend, &app.handle()).unwrap();
    }

    /// TEST HELPERS
    struct TestBackend {}

    impl Backend for TestBackend {
        fn enroll_user(&self) -> miette::Result<()> {
            Ok(())
        }

        fn reset(&self) -> miette::Result<()> {
            Ok(())
        }
    }
}
