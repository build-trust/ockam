use clap::Args;
use ockam_api::nodes::models::transport::DeleteTransport;
use ockam_core::api::Request;

use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{node::NodeOpts, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Tcp Connection ID
    pub id: String,

    /// Force this operation: delete the API transport if requested
    #[arg(long)]
    pub force: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (options, command): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let node_name = extract_address_value(&command.node_opts.api_node)?;
    let mut rpc = Rpc::background(&ctx, &options, &node_name)?;
    let request = Request::delete("/node/tcp/connection")
        .body(DeleteTransport::new(&command.id, command.force));
    rpc.request(request).await?;
    rpc.is_ok()?;

    println!("Tcp connection `{}` successfully deleted", command.id);
    Ok(())
}
