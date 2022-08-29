use clap::Args;

use ockam::Context;

use crate::node::NodeOpts;
use crate::util::api;
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Args, Clone, Debug)]
pub struct ListCommand {
    /// Node of which secure listeners shall be listed
    #[clap(flatten)]
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
    let mut rpc = Rpc::new(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::list_secure_channel_listener()).await?;
    let res = rpc.parse_response::<Vec<String>>()?;

    println!(
        "Secure channel listeners for node `{}`:",
        &cmd.node_opts.api_node
    );
    for addr in res {
        println!("  {}", addr);
    }

    Ok(())
}
