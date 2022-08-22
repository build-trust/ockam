use crate::util::{api, node_rpc, stop_node, ConfigError, Rpc};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::multiaddr_to_addr;
use ockam_api::nodes::models::secure_channel::DeleteSecureChannelResponse;
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

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }

    async fn rpc_callback(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        // We apply the inverse transformation done in the `create` command.
        let addr = multiaddr_to_addr(&self.channel)
            .ok_or_else(|| ConfigError::InvalidSecureChannelAddress(self.channel.to_string()))?;

        let mut rpc = Rpc::new(ctx, &opts, &self.node_opts.at)?;
        rpc.request(api::delete_secure_channel(&addr)).await?;
        let res = rpc.parse_response::<DeleteSecureChannelResponse>()?;
        match res.channel {
            Some(_) => println!("Deleted {}", self.channel),
            None => println!("Channel with address {} not found", self.channel),
        }
        Ok(())
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> crate::Result<()> {
    let res = cmd.rpc_callback(&ctx, opts).await;
    stop_node(ctx).await?;
    res
}
