use miette::miette;
use tauri::{AppHandle, Manager, Wry};
use tracing::log::info;

use ockam_command::{CommandGlobalOpts, GlobalArgs};

use crate::Result;

/// Reset the project.
/// This function removes all persisted state
/// So that the user must enroll again in order to be able to access a project
#[tauri::command]
pub fn reset(app: &AppHandle<Wry>) -> Result<()> {
    let options = CommandGlobalOpts::new(GlobalArgs::default());
    let res = if let Err(e) = options.state.delete(true) {
        Err(miette!("{:?}", e).into())
    } else {
        info!("Local Ockam configuration deleted");
        Ok(())
    };
    app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
    res
}
