use clap::Args;

use ockam::Context;

use crate::node::{get_node_name, NodeOpts};
use crate::util::api;
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

/// List Secure Channel Listeners
#[derive(Args, Clone, Debug)]
#[command(arg_required_else_help = true)]
pub struct ListCommand {
    /// Node of which secure listeners shall be listed
    #[command(flatten)]
    node_opts: NodeOpts,
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
    let node_name = get_node_name(&opts.state, cmd.node_opts.api_node.clone())?;
    let mut rpc = Rpc::background(ctx, &opts, &node_name)?;
    rpc.request(api::list_secure_channel_listener()).await?;
    let res = rpc.parse_response::<Vec<String>>()?;

    println!("Secure channel listeners for node `{}`:", &node_name);
    for addr in res {
        println!("  {addr}");
    }

    Ok(())
}
