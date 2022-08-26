use clap::Args;

use ockam::Context;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::api::{self};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct PresentCredentialCommand {
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(long, display_order = 900, name = "ROUTE")]
    pub to: MultiAddr,

    #[clap(short, long)]
    pub oneway: bool,
}

impl PresentCredentialCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: PresentCredentialCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, PresentCredentialCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: PresentCredentialCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::new(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::credentials::present_credential(&cmd.to, cmd.oneway))
        .await?;
    Ok(())
}
