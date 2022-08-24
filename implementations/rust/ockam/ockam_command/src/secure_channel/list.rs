use clap::Args;
use const_str::replace as const_replace;

use ockam::Context;

use crate::{
    node::NodeOpts,
    secure_channel::BACKGROUND,
    util::{api, node_rpc, stop_node, Rpc},
    CommandGlobalOpts, HELP_TEMPLATE,
};

/// List Secure Channels
#[derive(Clone, Debug, Args)]
#[clap(
    display_order = 900,
    help_template = const_replace!(HELP_TEMPLATE, "LEARN MORE", BACKGROUND)
)]
pub struct ListCommand {
    /// Node of which secure channels shall be listed
    #[clap(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(secure_channel_list_rpc, (opts, self));
    }
}

async fn secure_channel_list_rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    let res = secure_channel_list_rpc_impl(&mut ctx, opts, cmd).await;
    stop_node(ctx).await?;
    res
}

async fn secure_channel_list_rpc_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ListCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::new(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::list_secure_channels()).await?;
    let res = rpc.parse_response::<Vec<String>>()?;

    println!("Secure channels for node `{}`:", &cmd.node_opts.api_node);

    for addr in res {
        println!("  {}", addr);
    }

    Ok(())
}
