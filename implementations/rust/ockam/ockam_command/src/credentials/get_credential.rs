use clap::Args;

use ockam::Context;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::{api, node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct GetCredentialCommand {
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(long, short)]
    pub from: MultiAddr,

    #[clap(long)]
    pub overwrite: bool,
}

impl GetCredentialCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: GetCredentialCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, GetCredentialCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: GetCredentialCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::new(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::credentials::get_credential(&cmd.from, cmd.overwrite))
        .await?;
    Ok(())
}
