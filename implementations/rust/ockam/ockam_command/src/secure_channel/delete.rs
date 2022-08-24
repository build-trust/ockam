use crate::util::{api, node_rpc, stop_node, ConfigError, Rpc, Rpc1, CmdTrait};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::multiaddr_to_addr;
use ockam_api::nodes::models::secure_channel::{DeleteSecureChannelRequest, DeleteSecureChannelResponse};
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct SecureChannelNodeOpts {
    #[clap(
        global = true,
        short,
        long,
        value_name = "NODE",
        default_value = "default"
    )]
    pub at: String,
}

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[clap(flatten)]
    node_opts: SecureChannelNodeOpts,

    channel: MultiAddr,
}

impl<'a> CmdTrait<'a> for DeleteCommand {
    type Req = DeleteSecureChannelRequest<'a>;
    type Resp = DeleteSecureChannelResponse<'a>;

    fn req(&mut self) -> ockam_core::api::RequestBuilder<'a, Self::Req> {
        let addr = multiaddr_to_addr(&self.channel)
            .ok_or_else(|| ConfigError::InvalidSecureChannelAddress(self.channel.to_string())).unwrap();
        api::delete_secure_channel(&addr)
    }
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> crate::Result<()> {
    let res = rpc_callback(cmd, &ctx, opts).await;
    stop_node(ctx).await?;
    res
}

async fn rpc_callback(mut cmd: DeleteCommand, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
    // We apply the inverse transformation done in the `create` command.
    let at = cmd.node_opts.at.clone();
    let raw_res = Rpc1::new(ctx, &mut cmd, &opts, &at)?.request_then_response().await?;
    let _resp = cmd.parse_response(&raw_res)?;
//    let res = rpc.parse_response()?;
/*        match res.channel {
        Some(_) => println!("Deleted {}", self.channel),
        None => println!("Channel with address {} not found", self.channel),
    }*/
    Ok(())
}
