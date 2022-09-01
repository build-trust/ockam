use clap::Args;

use ockam::Context;

use crate::secure_channel::HELP_DETAIL;
use crate::{
    help,
    node::NodeOpts,
    util::{api, node_rpc, Rpc},
    CommandGlobalOpts,
};

/// List Secure Channels
#[derive(Clone, Debug, Args)]
#[clap(arg_required_else_help = true, help_template = help::template(HELP_DETAIL))]
pub struct ListCommand {
    /// Node at which the returned secure channels were initiated (required)
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
    secure_channel_list_rpc_impl(&mut ctx, opts, cmd).await
}

async fn secure_channel_list_rpc_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ListCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::background(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::list_secure_channels()).await?;
    let res = rpc.parse_response::<Vec<String>>()?;

    println!("Secure channels for node `{}`:", &cmd.node_opts.api_node);

    for addr in res {
        println!("  {}", addr);
    }

    Ok(())
}
