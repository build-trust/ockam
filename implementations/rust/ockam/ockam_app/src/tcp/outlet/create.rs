use tauri::{AppHandle, Manager, Wry};
use tracing::error;
use tracing::log::info;

use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::nodes::models::portal::CreateOutlet;
use ockam_command::tcp;
use ockam_command::util::{extract_address_value, get_free_address};

use crate::app::AppState;
use crate::Result;

/// Create a TCP outlet within the default node.
pub async fn create(app: &AppHandle<Wry>) -> Result<()> {
    info!("creating an outlet");
    let state = app.state::<AppState>();
    let to = get_free_address()?.to_string();
    let from = {
        let from = tcp::outlet::create::default_from_addr();
        extract_address_value(&from)?
    };
    let to_node = state.options().state.nodes.default()?.name().to_string();
    let payload = CreateOutlet::new(to, from, None, true);
    let res = tcp::outlet::create::send_request(
        &state.context(),
        &state.options(),
        payload,
        Some(to_node),
    )
    .await;
    match res {
        Err(e) => error!("{:?}", e),
        Ok(s) => info!("the outlet status is {:?}", s),
    }
    app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
    Ok(())
}
