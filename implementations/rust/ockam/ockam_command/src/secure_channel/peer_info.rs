use crate::shared_args::{IdentityOpts, TimeoutArg};
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use miette::IntoDiagnostic;
use ockam::{identity::Identifier, Context};
use ockam_api::nodes::service::SecureChannelType;
use ockam_api::nodes::InMemoryNode;
use ockam_api::output::Output;
use ockam_multiaddr::MultiAddr;
use serde::Serialize;

const HELP_DETAIL: &str = "";

#[derive(Debug, Clone, Serialize)]
#[rustfmt::skip]
pub struct PeerInfo {
    pub identifier: Identifier,
    pub change_history: String,
}

impl Output for PeerInfo {
    fn item(&self) -> ockam_api::Result<String> {
        Ok(format!(
            "\n Identifier: {}\n Change History: {}\n",
            self.identifier, self.change_history
        ))
    }
}

/// Retrieve Peer Identity
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct PeerInfoCommand {
    /// Route to a secure channel listener

    #[arg(value_name = "ROUTE", long, display_order = 800)]
    pub to: MultiAddr,

    #[command(flatten)]
    identity_opts: IdentityOpts,

    #[command(flatten)]
    pub timeout: TimeoutArg,
}

impl PeerInfoCommand {
    pub fn name(&self) -> String {
        "secure-channel peer-info".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let identity_name = opts
            .state
            .get_identity_name_or_default(&self.identity_opts.identity_name)
            .await?;

        let node_manager =
            InMemoryNode::start_node(ctx, &opts.state, &identity_name, None, None, None, None)
                .await?;

        let secure_channel = node_manager
            .create_secure_channel(
                ctx,
                self.to.clone(),
                Some(identity_name),
                None,
                None,
                Some(self.timeout.timeout),
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await?;

        let peer_identifier = secure_channel.their_identifier();

        let change_history = node_manager
            .secure_channels()
            .identities()
            .get_change_history(peer_identifier)
            .await?;
        let peer_info = PeerInfo {
            identifier: peer_identifier.to_owned(),
            change_history: change_history.export_as_string()?,
        };

        opts.terminal
            .to_stdout()
            .plain(peer_info.item()?)
            .json(serde_json::to_string(&peer_info).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}
