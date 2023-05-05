use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{node::NodeOpts, CommandGlobalOpts};
use clap::Args;
use ockam_api::nodes::models;
use ockam_core::api::Request;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Tcp Connection ID
    pub address: String,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let node_name = extract_address_value(&cmd.node_opts.api_node)?;

    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    let req = Request::delete("/node/tcp/connection")
        .body(models::transport::DeleteTransport::new(cmd.address.clone()));
    rpc.request(req).await?;
    rpc.is_ok()?;

    println!("Tcp connection `{}` successfully deleted", cmd.address);
    Ok(())
}
