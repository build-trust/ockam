use clap::Args;

use ockam::Context;
use ockam_multiaddr::MultiAddr;

use crate::node::util::delete_embedded_node;
use crate::util::{api, node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct GetCredentialCommand {
    #[clap(long, short)]
    pub from: MultiAddr,

    #[clap(long)]
    pub overwrite: bool,
}

impl GetCredentialCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
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
    let mut rpc = Rpc::embedded(ctx, &opts).await?;
    rpc.request(api::credentials::get_credential(&cmd.from, cmd.overwrite))
        .await?;
    delete_embedded_node(&opts.config, rpc.node_name()).await;
    Ok(())
}
