use core::time::Duration;

use clap::Args;
use miette::{Context as _, IntoDiagnostic};

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::secure_channel::CredentialExchangeMode;
use ockam_api::nodes::service::message::SendMessage;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::node::util::delete_embedded_node;
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::duration::duration_parser;
use crate::util::{clean_nodes_multiaddr, node_rpc, Rpc};
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
    #[arg(short, long, value_name = "NODE")]
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
        initialize_identity_if_default(&opts, &self.cloud_opts.identity);
        node_rpc(rpc, (opts, self))
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, SendCommand),
) -> miette::Result<()> {
    async fn go(
        ctx: &mut Context,
        opts: CommandGlobalOpts,
        cmd: SendCommand,
    ) -> miette::Result<()> {
        // Setup environment depending on whether we are sending the message from an embedded node or a background node
        let mut rpc = if let Some(node) = &cmd.from {
            let api_node = extract_address_value(node)?;
            Rpc::background(ctx, &opts, &api_node).await?
        } else {
            let identity = get_identity_name(&opts.state, &cmd.cloud_opts.identity);
            Rpc::embedded_with_vault_and_identity(ctx, &opts, identity, &cmd.trust_context_opts)
                .await?
        };

        // Process `--to` Multiaddr
        let (to, meta) =
            clean_nodes_multiaddr(&cmd.to, &opts.state).context("Argument '--to' is invalid")?;

        // Replace `/project/<name>` occurrences with their respective secure channel addresses
        let projects_sc = crate::project::util::get_projects_secure_channels_from_config_lookup(
            &opts,
            &mut rpc,
            &meta,
            CredentialExchangeMode::Oneway,
        )
        .await?;
        let to = crate::project::util::clean_projects_multiaddr(to, projects_sc)?;

        let msg_bytes = if cmd.hex {
            hex::decode(cmd.message)
                .into_diagnostic()
                .context("The message is not a valid hex string")?
        } else {
            cmd.message.as_bytes().to_vec()
        };

        // Send request
        let response: Vec<u8> = rpc
            .set_timeout(cmd.timeout)
            .ask(req(&to, msg_bytes))
            .await?;
        let result = if cmd.hex {
            hex::encode(response)
        } else {
            String::from_utf8(response)
                .into_diagnostic()
                .context("Received content is not a valid utf8 string")?
        };

        // only delete node in case 'from' is empty and embedded node was started before
        if cmd.from.is_none() {
            delete_embedded_node(&opts, rpc.node_name()).await;
        }
        opts.terminal.stdout().plain(&result).write_line()?;
        Ok(())
    }
    go(&mut ctx, opts, cmd).await
}

pub(crate) fn req(to: &MultiAddr, message: Vec<u8>) -> Request<SendMessage> {
    Request::post("v0/message").body(SendMessage::new(to, message))
}
