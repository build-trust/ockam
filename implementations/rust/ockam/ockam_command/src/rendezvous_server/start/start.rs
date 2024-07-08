use crate::util::foreground_args::ForegroundArgs;
use crate::util::{embedded_node_that_is_not_stopped, local_cmd};
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use ockam_node::Context;
use tracing::instrument;

/// Start a Rendezvous server in foreground
#[derive(Clone, Debug, Args)]
pub struct StartCommand {
    #[command(flatten)]
    pub foreground_args: ForegroundArgs,

    /// The address to bind the UDP listener to.
    #[arg(
        display_order = 900,
        long = "udp",
        id = "UDP_SOCKET_ADDRESS",
        default_value = "0.0.0.0:4000"
    )]
    pub udp_address: String,

    /// The address to bind the TCP listener to support healthcheck.
    #[arg(
        display_order = 900,
        long = "healthcheck",
        id = "TCP_SOCKET_ADDRESS",
        default_value = "0.0.0.0:4001"
    )]
    pub healthcheck_address: String,
}

#[async_trait]
impl Command for StartCommand {
    const NAME: &'static str = "rendezvous-server start";

    #[instrument(skip_all)]
    fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        local_cmd(embedded_node_that_is_not_stopped(
            opts.rt.clone(),
            |ctx| async move { self.foreground_mode(&ctx, opts).await },
        ))
    }

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        self.foreground_mode(ctx, opts).await?;

        Ok(())
    }
}
