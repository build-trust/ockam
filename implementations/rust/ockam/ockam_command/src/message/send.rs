use async_trait::async_trait;
use clap::Args;
use miette::{Context as _, IntoDiagnostic};
use tracing::info;

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::service::messages::Messages;
use ockam_api::nodes::InMemoryNode;
use ockam_api::nodes::{BackgroundNodeClient, NodeManager};
use ockam_multiaddr::MultiAddr;

use crate::project::util::{
    clean_projects_multiaddr, get_projects_secure_channels_from_config_lookup,
};
use crate::shared_args::{IdentityOpts, TrustOpts};
use crate::shared_args::{RetryOpts, TimeoutArg};
use crate::util::clean_nodes_multiaddr;
use crate::{docs, Command, CommandGlobalOpts, Error};

const LONG_ABOUT: &str = include_str!("./static/send/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/send/after_long_help.txt");

#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
about = docs::about("Send a message to an Ockam node"),
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct SendCommand {
    pub message: String,

    /// The node to send messages from
    #[arg(short, long, value_name = "NODE", value_parser = extract_address_value)]
    from: Option<String>,

    /// The route to send the message to
    #[arg(short, long, value_name = "ROUTE")]
    pub to: MultiAddr,

    /// Flag to indicate that the message is hex encoded
    #[arg(long)]
    pub hex: bool,

    #[command(flatten)]
    pub timeout: TimeoutArg,

    #[command(flatten)]
    pub retry_opts: RetryOpts,

    #[command(flatten)]
    identity_opts: IdentityOpts,

    #[command(flatten)]
    pub trust_opts: TrustOpts,
}

#[async_trait]
impl Command for SendCommand {
    const NAME: &'static str = "message send";

    fn retry_opts(&self) -> Option<RetryOpts> {
        Some(self.retry_opts.clone())
    }

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        // Process `--to` Multiaddr
        let (to, meta) = clean_nodes_multiaddr(&self.to, &opts.state)
            .await
            .context("Argument '--to' is invalid")
            .map_err(Error::Retry)?;

        // Setup environment depending on whether we are sending the message from a background node
        // or an in-memory node
        let result = if let Some(node) = &self.from {
            let client = BackgroundNodeClient::create_to_node(ctx, &opts.state, node.as_str())?;
            self.send_message(&client, ctx, &to).await?
        } else {
            let identity_name = opts
                .state
                .get_identity_name_or_default(&self.identity_opts.identity_name)
                .await?;

            info!("starting an in memory node to send a message");

            let node_manager = InMemoryNode::start_node(
                ctx,
                &opts.state,
                &identity_name,
                None,
                self.trust_opts.project_name.clone(),
                self.trust_opts.authority_identity.clone(),
                self.trust_opts.authority_route.clone(),
            )
            .await?;
            info!("started an in memory node to send a message");

            // Replace `/project/<name>` occurrences with their respective secure channel addresses
            let projects_sc = get_projects_secure_channels_from_config_lookup(
                &opts,
                ctx,
                &node_manager,
                &meta,
                Some(identity_name),
                Some(self.timeout.timeout),
            )
            .await
            .context("Failed to resolve projects from '--to' address")
            .map_err(Error::Retry)?;
            let to = clean_projects_multiaddr(to, projects_sc)?;
            info!("sending to {to}");
            let n: &NodeManager = &node_manager;
            self.send_message(n, ctx, &to).await?
        };

        opts.terminal.stdout().plain(result).write_line()?;
        Ok(())
    }
}

impl SendCommand {
    async fn send_message(
        self,
        client: &impl Messages,
        ctx: &Context,
        to: &MultiAddr,
    ) -> crate::Result<String> {
        if self.hex {
            let to_send = hex::decode(self.message.clone())
                .into_diagnostic()
                .context("The message is not a valid hex string")?;
            let response: Vec<u8> = client
                .send_message(ctx, to, to_send, Some(self.timeout.timeout))
                .await
                .map_err(Error::Retry)?;
            Ok(hex::encode(response))
        } else {
            client
                .send_message(ctx, to, self.message, Some(self.timeout.timeout))
                .await
                .map_err(Error::Retry)
                .into_diagnostic()
                .context("Received content is not a valid utf8 string")
        }
    }
}
