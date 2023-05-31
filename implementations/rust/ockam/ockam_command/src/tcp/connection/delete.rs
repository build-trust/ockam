use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{docs, node::NodeOpts, CommandGlobalOpts};
use clap::Args;
use ockam_api::nodes::models;
use ockam_core::api::Request;

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP connection
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Tcp Connection ID
    pub address: String,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.api_node);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.api_node);
    let node_name = extract_address_value(&node_name)?;

    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    let req = Request::delete("/node/tcp/connection")
        .body(models::transport::DeleteTransport::new(cmd.address.clone()));
    rpc.request(req).await?;
    rpc.is_ok()?;

    println!("Tcp connection `{}` successfully deleted", cmd.address);
    Ok(())
}
