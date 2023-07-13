use crate::app::State;
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
    // Update tray menu
    let state = app_handle.state::<State>();
    {
        let tray_handle = app_handle.tray_handle();
        let mut tray_menu = state.tray_menu.write().unwrap();
        tray_menu.options.reset.set_enabled(&tray_handle, false);
        tray_menu.enroll.enroll.set_enabled(&tray_handle, true);
        tray_menu.refresh(&tray_handle);
    }
    res
}
