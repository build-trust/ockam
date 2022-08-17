use clap::Args;

use ockam::identity::IdentityIdentifier;
use ockam::Context;
use ockam_api::authenticator::direct::types::AddMember;
use ockam_api::clean_multiaddr;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::{node_rpc, RpcBuilder};
use crate::{stop_node, CommandGlobalOpts, Result};

#[derive(Clone, Debug, Args)]
pub struct AddMemberCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(long, short)]
    member: IdentityIdentifier,

    #[clap(long, short)]
    to: MultiAddr,
}

impl AddMemberCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, AddMemberCommand)) -> Result<()> {
    async fn go(ctx: &mut Context, opts: &CommandGlobalOpts, cmd: AddMemberCommand) -> Result<()> {
        let to = clean_multiaddr(&cmd.to, &opts.config.get_lookup()).unwrap();
        let req = Request::post("/members").body(AddMember::new(cmd.member));
        let mut rpc = RpcBuilder::new(ctx, opts, &cmd.node_opts.api_node)
            .to(&to)?
            .build()?;
        rpc.request(req).await?;
        rpc.is_ok()
    }
    let result = go(&mut ctx, &opts, cmd).await;
    stop_node(ctx).await?;
    result
}
