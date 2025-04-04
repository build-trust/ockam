use async_trait::async_trait;
use clap::Args;
use miette::miette;
use std::sync::Arc;

use ockam::Context;
use ockam_api::nodes::models::secure_channel::ShowSecureChannelResponse;
use ockam_api::nodes::service::SecureChannelType;
use ockam_api::nodes::{BackgroundNodeClient, InMemoryNode};
use ockam_api::{CliState, ReverseLocalConverter};
use ockam_core::Address;

use crate::shared_args::{IdentityOpts, TimeoutArg};
use crate::{docs, util::api, Command, CommandGlobalOpts};
use ockam_api::output::Output;
use ockam_multiaddr::proto::{Node, Service};
use ockam_multiaddr::{MultiAddr, Protocol};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show Secure Channels
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ShowCommand {
    /// Route to the secure channel
    #[arg(value_name = "ROUTE", long)]
    at: MultiAddr,

    #[command(flatten)]
    identity_opts: IdentityOpts,

    #[command(flatten)]
    pub timeout: TimeoutArg,
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "secure-channel show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let response =
            match extract_node_name_and_service_from_multiaddr(&self.at, opts.state.clone()).await?
            {
                // Get the secure channel from a local node
                Some((node_name, sc_address)) => {
                    let node =
                        BackgroundNodeClient::create(ctx, opts.state.clone(), &Some(node_name))
                            .await?;
                    let response: ShowSecureChannelResponse =
                        node.ask(ctx, api::show_secure_channel(&sc_address)).await?;
                    response
                }
                // Get the secure channel given a multiaddr
                None => {
                    let identity = opts
                        .state
                        .get_named_identity_or_default(&self.identity_opts.identity_name)
                        .await?;

                    let node = InMemoryNode::start_with_identity(
                        ctx,
                        opts.state.clone(),
                        Some(identity.name()),
                    )
                    .await?;

                    let secure_channel = node
                        .create_secure_channel(
                            self.at.clone(),
                            Some(identity.name()),
                            None,
                            None,
                            Some(self.timeout.timeout),
                            SecureChannelType::KeyExchangeAndMessages,
                        )
                        .await?;

                    let peer_identifier = secure_channel.their_identifier();

                    let change_history = node
                        .secure_channels()
                        .identities()
                        .get_change_history(peer_identifier)
                        .await?;

                    ShowSecureChannelResponse {
                        address: ReverseLocalConverter::convert_address(
                            secure_channel.encryptor_address(),
                        )?,
                        route: self.at,
                        authorized_identifiers: None,
                        flow_control_id: secure_channel.flow_control_id().clone(),
                        their_identifier: secure_channel.their_identifier().clone(),
                        their_change_history: Some(change_history.export_as_string()?),
                    }
                }
            };

        opts.terminal
            .to_stdout()
            .plain(response.item()?)
            .json_obj(response)?
            .write_line()?;
        Ok(())
    }
}

/// Extracts the node name and service address from a multiaddr starting with a node protocol.
/// Returns None if the multiaddr does not start with a node protocol.
///
/// Example:
///     `/node/n1/service/1234` -> `(n1, 1234)`
async fn extract_node_name_and_service_from_multiaddr(
    addr: &MultiAddr,
    cli_state: Arc<CliState>,
) -> miette::Result<Option<(String, Address)>> {
    let mut iter = addr.iter();
    if let Some(proto) = iter.next() {
        if proto.code() == Node::CODE {
            let alias = proto
                .cast::<Node>()
                .ok_or_else(|| miette!("Invalid node address protocol"))?;
            let node_info = cli_state.get_node(&alias).await?;
            let node_name = node_info.name();
            if let Some(proto) = iter.next() {
                if proto.code() == Service::CODE {
                    let alias = proto
                        .cast::<Service>()
                        .ok_or_else(|| miette!("Invalid node address protocol"))?;
                    let address = Address::from_string(alias.to_string());
                    return Ok(Some((node_name, address)));
                }
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use miette::Result;
    use ockam_api::CliState;
    use ockam_core::Address;
    use ockam_multiaddr::MultiAddr;

    #[tokio::test]
    async fn test_extract_node_name_and_service_from_multiaddr_valid() -> Result<()> {
        let cli_state = Arc::new(CliState::test().await?);
        cli_state.create_node("n1").await?;

        let multiaddr: MultiAddr = "/node/n1/service/1234".parse()?;

        let result = extract_node_name_and_service_from_multiaddr(&multiaddr, cli_state).await?;
        assert!(result.is_some());
        let (node_name, address) = result.unwrap();
        assert_eq!(node_name, "n1");
        assert_eq!(address, Address::from_string("1234".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_extract_node_name_and_service_from_multiaddr_invalid_node() -> Result<()> {
        let cli_state = Arc::new(CliState::test().await?);
        let multiaddr: MultiAddr = "/node/invalid/service/1234".parse()?;

        let result = extract_node_name_and_service_from_multiaddr(&multiaddr, cli_state).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_extract_node_name_and_service_from_multiaddr_no_node() -> Result<()> {
        let cli_state = Arc::new(CliState::test().await?);
        let multiaddr: MultiAddr = "/service/1234".parse()?;

        let result = extract_node_name_and_service_from_multiaddr(&multiaddr, cli_state).await?;
        assert!(result.is_none());
        Ok(())
    }
}
