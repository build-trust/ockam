use tauri::{AppHandle, Manager, Wry};

use crate::app::AppState;
use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::nodes::models::portal::{CreateOutlet, OutletStatus};
use ockam_command::node::initialize_node_if_default;
use ockam_command::util::{embedded_node, extract_address_value, get_free_address};
use ockam_command::{tcp, CommandGlobalOpts};

use crate::Result;

/// Create a TCP outlet within the default node.
#[tauri::command]
pub fn create(app: &AppHandle<Wry>) -> Result<()> {
    let options = app.state::<AppState>().options();
    initialize_node_if_default(&options, &None);
    embedded_node(rpc, options.clone())?;
    app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
    Ok(())
}

async fn rpc(ctx: Context, options: CommandGlobalOpts) -> miette::Result<OutletStatus> {
    let to = get_free_address()?.to_string();
    let from = {
        let from = tcp::outlet::create::default_from_addr();
        extract_address_value(&from)?
    };
    let to_node = options.state.nodes.default()?.name().to_string();
    let payload = CreateOutlet::new(to, from, None, true);
    let res = tcp::outlet::create::send_request(&ctx, &options, payload, Some(to_node)).await?;
    Ok(res)
}
