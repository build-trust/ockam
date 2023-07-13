use crate::{AppHandle, Result};
use miette::{miette, IntoDiagnostic};
use ockam_command::{CommandGlobalOpts, GlobalArgs};
use tauri::Manager;

/// Reset the project.
/// This function removes all persisted state
/// So that the user must enroll again in order to be able to access a project
#[tauri::command]
pub fn reset(app_handle: AppHandle) -> Result<()> {
    let options = CommandGlobalOpts::new(GlobalArgs::default());
    let res = if let Err(e) = options.state.delete(true) {
        Err(miette!("{:?}", e).into())
    } else {
        options
            .terminal
            .write_line("Local Ockam configuration deleted")
            .into_diagnostic()?;
        Ok(())
    };
    app_handle.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
    res
}
