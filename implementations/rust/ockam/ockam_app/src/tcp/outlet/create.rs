use tauri::{AppHandle, Manager, Wry};
use tracing::error;
use tracing::log::info;

use ockam_command::tcp;
use ockam_command::util::{extract_address_value, get_free_address};
use ockam_core::api::Id;

use crate::app::AppState;
use crate::Result;

/// Create a TCP outlet within the default node.
pub async fn tcp_outlet_create(app: &AppHandle<Wry>) -> Result<()> {
    info!("creating an outlet");
    let state = app.state::<AppState>();
    let to = get_free_address()?.to_string();
    let from = {
        let from = tcp::outlet::create::default_from_addr();
        extract_address_value(&from)?
    };
    let res = state
        .node_manager()
        .create_outlet_impl(&state.context(), Id::fresh(), to, from, None, true)
        .await;
    match res {
        Err(e) => error!("{:?}", e),
        Ok(s) => info!("the outlet status is {:?}", s),
    }

    app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);

    Ok(())
}
