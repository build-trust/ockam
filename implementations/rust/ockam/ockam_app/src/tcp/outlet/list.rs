use crate::error::TauriCommandResult;
use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_command::node::initialize_node_if_default;
use ockam_command::util::embedded_node;
use ockam_command::{tcp, CommandGlobalOpts, GlobalArgs};
use tauri::Manager;

/// List TCP outlets of the default node.
#[tauri::command]
pub fn tcp_outlet_list() -> TauriCommandResult<Vec<OutletStatus>> {
    let opts = CommandGlobalOpts::new(GlobalArgs::default());
    initialize_node_if_default(&opts, &None);
    match embedded_node(rpc, opts) {
        Ok(res) => Ok(res),
        Err(e) => Err(format!("Error while listing TCP outlets: {e:?}")),
    }
}

async fn rpc(ctx: Context, opts: CommandGlobalOpts) -> miette::Result<Vec<OutletStatus>> {
    let to_node = opts.state.nodes.default()?.name().to_string();
    let res = tcp::outlet::list::send_request(&ctx, &opts, Some(to_node)).await?;
    Ok(res.list)
}
