use clap::Args;
use ockam::{Context, TcpTransport};

use ockam_api::nodes::models::services::ServiceList;

use crate::util::{api, node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;

/// List service(s) of a given node
#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    /// Name of the node.
    #[arg(display_order = 900)]
    pub node_name: String,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ListCommand,
) -> crate::Result<()> {
    let node_name = cmd.node_name;
    let tcp = TcpTransport::create(&ctx).await?;

    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).tcp(&tcp)?.build();
    rpc.request(api::list_services()).await?;
    rpc.parse_and_print_response::<ServiceList>()?;

    Ok(())
}
