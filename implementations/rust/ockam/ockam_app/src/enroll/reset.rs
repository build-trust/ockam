use miette::{miette, IntoDiagnostic};

use ockam_command::{CommandGlobalOpts, GlobalArgs};

/// Reset the project.
/// This function removes all persisted state
/// So that the user must enroll again in order to be able to access a project
#[tauri::command]
pub fn reset() -> miette::Result<()> {
    let options = CommandGlobalOpts::new(GlobalArgs::default());
    if let Err(e) = options.state.delete(true) {
        Err(miette!("{:?}", e))
    } else {
        options
            .terminal
            .write_line("Local Ockam configuration deleted")
            .into_diagnostic()?;
        Ok(())
    }
}
