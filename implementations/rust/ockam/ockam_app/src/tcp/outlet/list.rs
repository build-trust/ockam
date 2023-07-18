use tauri::{AppHandle, Manager, Wry};

use ockam_api::nodes::models::portal::OutletList;
use ockam_command::{tcp, CommandGlobalOpts};

use crate::app::AppState;
use crate::Result;

/// List TCP outlets of the default node.
pub async fn list(app: &AppHandle<Wry>, options: &CommandGlobalOpts) -> Result<OutletList> {
    let context = app.state::<AppState>().context();
    tcp::outlet::list::send_request(&context, &options, None)
        .await
        .map_err(|e| e.into())
}
