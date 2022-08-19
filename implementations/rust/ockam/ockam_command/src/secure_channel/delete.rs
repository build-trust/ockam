use crate::util::{api, node_rpc, stop_node, Rpc};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::models::secure_channel::DeleteSecureChannelResponse;

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

    #[clap(default_value = "default")]
    channel: String,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }

    async fn rpc_callback(
        self,
        ctx: &ockam::Context,
        opts: CommandGlobalOpts,
    ) -> crate::Result<()> {
        let ch = self.channel.clone();

        let mut rpc = Rpc::new(ctx, &opts, &self.node_opts.at)?;
        rpc.request(api::delete_secure_channel(self.channel.into()))
            .await?;
        let res = rpc.parse_response::<DeleteSecureChannelResponse>()?;

        match res.channel {
            Some(_) => println!("deleted {}", ch),
            None => println!("channel with address {} not found", ch),
        }
        Ok(())
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> crate::Result<()> {
    let res = cmd.rpc_callback(&ctx, opts).await;
    stop_node(ctx).await?;
    res
}
