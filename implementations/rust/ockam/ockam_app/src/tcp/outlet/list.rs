use crate::ctx::TauriCtx;
use crate::Result;
use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::nodes::models::portal::OutletList;
use ockam_command::node::initialize_node_if_default;
use ockam_command::util::embedded_node;
use ockam_command::{tcp, CommandGlobalOpts};

/// List TCP outlets of the default node.
#[tauri::command]
pub fn list(_ctx: &TauriCtx, options: &CommandGlobalOpts) -> Result<OutletList> {
    initialize_node_if_default(options, &None);
    let res = embedded_node(rpc, options.clone())?;
    Ok(res)
}

async fn rpc(ctx: Context, options: CommandGlobalOpts) -> miette::Result<OutletList> {
    let to_node = options.state.nodes.default()?.name().to_string();
    let res = tcp::outlet::list::send_request(&ctx, &options, Some(to_node)).await?;
    Ok(res)
}
