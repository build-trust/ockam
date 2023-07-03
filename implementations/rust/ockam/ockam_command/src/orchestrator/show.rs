use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{fmt_info, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use minicbor::{Decode, Decoder, Encode};
use ockam::identity::{SecureChannelOptions, TrustIdentifierPolicy};
use ockam::{route, Context, MessageSendReceiveOptions, Node, TcpConnectionOptions, TcpTransport};
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::NodeManager;
use ockam_core::api::{Request, Response, Status};
use std::time::Duration;

/// Retrieve the version information from Orchestrator nodes
#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Override default timeout (in seconds)
    #[arg(long, default_value = "30")]
    pub timeout: u64,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(get_orchestrator_version, (options, self))
    }
}

async fn get_orchestrator_version(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    let controller_addr = CloudOpts::route();
    let controller_tcp_addr = controller_addr.to_socket_addr().into_diagnostic()?;
    let controller_identifier = NodeManager::load_controller_identifier().into_diagnostic()?;
    let tcp = TcpTransport::create(&ctx).await.into_diagnostic()?;
    let connection = tcp
        .connect(controller_tcp_addr, TcpConnectionOptions::new())
        .await
        .into_diagnostic()?;
    let node = {
        let identities_vault = opts.state.vaults.default()?.get().await?;
        let identities_repository = opts.state.identities.identities_repository().await?;
        Node::builder()
            .with_identities_vault(identities_vault)
            .with_identities_repository(identities_repository)
            .build(ctx)
            .await
            .into_diagnostic()?
    };
    let identifier = opts.state.identities.default()?.identifier();
    let secure_channel_options = SecureChannelOptions::new()
        .with_trust_policy(TrustIdentifierPolicy::new(controller_identifier))
        .with_timeout(Duration::from_secs(cmd.timeout));
    let secure_channel = node
        .create_secure_channel(
            &identifier,
            route![connection, "api"],
            secure_channel_options,
        )
        .await
        .into_diagnostic()?;

    // Send request
    let buf: Vec<u8> = node
        .send_and_receive_extended::<Vec<u8>>(
            route![secure_channel, "version_info"],
            Request::get("").to_vec().into_diagnostic()?,
            MessageSendReceiveOptions::new().with_timeout(Duration::from_secs(cmd.timeout)),
        )
        .await
        .into_diagnostic()?
        .body();
    let mut dec = Decoder::new(&buf);
    let hdr = dec.decode::<Response>().into_diagnostic()?;
    if hdr.status() == Some(Status::Ok) {
        let info = dec.decode::<VersionInfo>().into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(
                fmt_info!("Controller version: {}\n", info.controller_version)
                    + &fmt_info!("Project version: {}", info.project_version),
            )
            .json(serde_json::to_string(&info).into_diagnostic()?)
            .write_line()?;
        Ok(())
    } else {
        Err(miette!("Failed to retrieve version information from node."))
    }
}

#[derive(Encode, Decode, Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(Clone))]
#[cbor(map)]
pub struct VersionInfo {
    #[cfg(feature = "tag")]
    #[n(0)]
    #[serde(skip)]
    pub tag: TypeTag<3783607>,
    #[n(1)]
    pub controller_version: String,
    #[n(2)]
    pub project_version: String,
}
