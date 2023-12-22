use core::time::Duration;

use clap::Args;
use miette::{Context as _, IntoDiagnostic};
use tracing::info;

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::service::message::{MessageSender, SendMessage};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::nodes::InMemoryNode;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::project::util::{
    clean_projects_multiaddr, get_projects_secure_channels_from_config_lookup,
};
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::duration::duration_parser;
use crate::util::{clean_nodes_multiaddr, node_rpc};
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

    /// Override default timeout
    #[arg(long, value_name = "TIMEOUT", default_value = "10s", value_parser = duration_parser)]
    pub timeout: Duration,

    pub message: String,

    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    pub trust_context_opts: TrustContextOpts,
}

impl SendCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self))
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, SendCommand)) -> miette::Result<()> {
    async fn go(ctx: &Context, opts: CommandGlobalOpts, cmd: SendCommand) -> miette::Result<()> {
        // Process `--to` Multiaddr
        let (to, meta) = clean_nodes_multiaddr(&cmd.to, &opts.state)
            .await
            .context("Argument '--to' is invalid")?;

        let msg_bytes = if cmd.hex {
            hex::decode(cmd.message)
                .into_diagnostic()
                .context("The message is not a valid hex string")?
        } else {
            cmd.message.as_bytes().to_vec()
        };

        // Setup environment depending on whether we are sending the message from a background node
        // or an in-memory node
        let response: Vec<u8> = if let Some(node) = &cmd.from {
            BackgroundNodeClient::create_to_node(ctx, &opts.state, node.as_str())
                .await?
                .set_timeout(cmd.timeout)
                .ask(ctx, req(&to, msg_bytes))
                .await?
        } else {
            let identity_name = opts
                .state
                .get_identity_name_or_default(&cmd.cloud_opts.identity)
                .await?;

            info!("retrieving the trust context");

            let named_trust_context = opts
                .state
                .retrieve_trust_context(
                    &cmd.trust_context_opts.trust_context,
                    &cmd.trust_context_opts.project_name,
                    &None,
                    &None,
                )
                .await?;
            info!("retrieved the trust context: {named_trust_context:?}");

            info!("starting an in memory node to send a message");

            let node_manager = InMemoryNode::start_node(
                ctx,
                &opts.state,
                &identity_name,
                cmd.trust_context_opts.project_name,
                named_trust_context,
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
                Some(cmd.timeout),
            )
            .await?;
            let to = clean_projects_multiaddr(to, projects_sc)?;
            info!("sending to {to}");
            node_manager
                .send_message(ctx, &to, msg_bytes, Some(cmd.timeout))
                .await
                .into_diagnostic()?
        };

        let result = if cmd.hex {
            hex::encode(response)
        } else {
            String::from_utf8(response)
                .into_diagnostic()
                .context("Received content is not a valid utf8 string")?
        };

        opts.terminal.stdout().plain(result).write_line()?;
        Ok(())
    }
    go(&ctx, opts, cmd).await
}

pub(crate) fn req(to: &MultiAddr, message: Vec<u8>) -> Request<SendMessage> {
    Request::post("v0/message").body(SendMessage::new(to, message))
}
