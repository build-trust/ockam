use clap::Args;

use ockam::Context;
use ockam_multiaddr::MultiAddr;

use crate::node::util::delete_embedded_node;
use crate::util::api::{self};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct PresentCredentialCommand {
    #[clap(long, display_order = 900, name = "ROUTE")]
    pub to: MultiAddr,

    #[clap(short, long)]
    pub oneway: bool,
}

impl PresentCredentialCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
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
    let mut rpc = Rpc::embedded(ctx, &opts).await?;
    rpc.request(api::credentials::present_credential(&cmd.to, cmd.oneway))
        .await?;
    delete_embedded_node(&opts.config, rpc.node_name()).await;
    Ok(())
}
