use tracing;
use tracing::{error, info};

use ockam_command::{CommandGlobalOpts, GlobalArgs};

/// Reset the project.
/// This function removes all persisted state
/// So that the user must enroll again in order to be able to access a project
///
#[tauri::command]
pub fn reset() {
    let options = CommandGlobalOpts::new(GlobalArgs::default());
    if let Err(e) = options.state.delete(true) {
        error!("{:?}", e)
    } else {
        info!("Local Ockam configuration deleted")
    }
}
