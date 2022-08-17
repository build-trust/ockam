use clap::Args;

use ockam::credential::Credential;
use ockam::Context;
use ockam_api::clean_multiaddr;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::{node_rpc, Rpc};
use crate::{stop_node, CommandGlobalOpts, Result};

#[derive(Clone, Debug, Args)]
pub struct GetCredentialCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(long, short)]
    to: MultiAddr,
}

impl GetCredentialCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, GetCredentialCommand),
) -> Result<()> {
    async fn go(
        ctx: &mut Context,
        opts: &CommandGlobalOpts,
        cmd: &GetCredentialCommand,
    ) -> Result<()> {
        let to = clean_multiaddr(&cmd.to, &opts.config.get_lookup()).unwrap();
        let mut rpc = Rpc::new(ctx, opts, &cmd.node_opts.api_node)?;
        rpc.request_to(Request::post("/credential"), &to).await?;
        rpc.print_response::<Credential<'_>>()
    }
    let result = go(&mut ctx, &opts, &cmd).await;
    stop_node(ctx).await?;
    result
}
