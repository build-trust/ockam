use clap::Args;
use miette::{Context as _, IntoDiagnostic};
use tracing::info;

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::service::messages::Messages;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::nodes::InMemoryNode;
use ockam_multiaddr::MultiAddr;

use crate::project::util::{
    clean_projects_multiaddr, get_projects_secure_channels_from_config_lookup,
};
use crate::shared_args::TimeoutArg;
use crate::shared_args::{IdentityOpts, TrustOpts};
use crate::util::{async_cmd, clean_nodes_multiaddr};
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/send/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/send/after_long_help.txt");

/// Send a message to an Ockam node
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct SendCommand {
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

    pub message: String,

    #[command(flatten)]
    identity_opts: IdentityOpts,

    #[command(flatten)]
    pub trust_opts: TrustOpts,
}

impl SendCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "message send".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        // Process `--to` Multiaddr
        let (to, meta) = clean_nodes_multiaddr(&self.to, &opts.state)
            .await
            .context("Argument '--to' is invalid")?;

        let msg_bytes = if self.hex {
            hex::decode(self.message.clone())
                .into_diagnostic()
                .context("The message is not a valid hex string")?
        } else {
            self.message.as_bytes().to_vec()
        };

        // Setup environment depending on whether we are sending the message from a background node
        // or an in-memory node
        let response: Vec<u8> = if let Some(node) = &self.from {
            BackgroundNodeClient::create_to_node(ctx, &opts.state, node.as_str())
                .await?
                .send_message(ctx, &to, msg_bytes, Some(self.timeout.timeout))
                .await?
        } else {
            let identity_name = opts
                .state
                .get_identity_name_or_default(&self.identity_opts.identity)
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
            .await?;
            let to = clean_projects_multiaddr(to, projects_sc)?;
            info!("sending to {to}");
            node_manager
                .send_message(ctx, &to, msg_bytes, Some(self.timeout.timeout))
                .await?
        };

        let result = if self.hex {
            hex::encode(response)
        } else {
            String::from_utf8(response)
                .into_diagnostic()
                .context("Received content is not a valid utf8 string")?
        };

        opts.terminal.stdout().plain(result).write_line()?;
        Ok(())
    }
}
