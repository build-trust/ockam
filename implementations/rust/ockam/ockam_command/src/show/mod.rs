use clap::Args;
use ockam_multiaddr::MultiAddr;

use std::str::FromStr;
use std::time::Duration;

use anyhow::anyhow;

use ockam_core::api::{Request, Response, Status};
use ockam_core::CowStr;

use minicbor::{Decode, Decoder, Encode};

use crate::node::util::start_embedded_node;
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::orchestrator_api::{OrchestratorApiBuilder, OrchestratorEndpoint};
use crate::util::{node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;

use crate::Result;
use ockam::identity::{
    IdentityIdentifier, SecureChannelOptions, TrustEveryonePolicy, TrustIdentifierPolicy,
};
use ockam::{node, route, Context, MessageSendReceiveOptions, TcpConnectionOptions, TcpTransport};

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[cbor(map)]
pub struct VersionInfo<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    pub tag: TypeTag<3783607>,
    #[b(1)]
    pub version: CowStr<'a>,
    #[b(2)]
    pub project_version: CowStr<'a>,
}

/// Send messages
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct ShowCommand {
    // TODO: Can make this optional param if wanted
    // #[arg(long, required = true, value_name = "TCP_ADDRESS")]
    // pub tcp_address: String,
    /// Node identity id
    #[arg(long, value_name = "REMOTE_IDENTITY_ID")]
    pub remote_identity_id: Option<IdentityIdentifier>,

    /// Override Default Timeout
    #[arg(long, value_name = "TIMEOUT", default_value = "10000")]
    pub timeout: u64,

    #[command(flatten)]
    pub trust_context_opts: TrustContextOpts,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(get_orchestrator_version, (options, self))
    }
}

async fn get_orchestrator_version(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> Result<()> {
    let mut orchestrator_api_builder =
        OrchestratorApiBuilder::new(&ctx, &opts, &cmd.trust_context_opts);

    if let Some(identity_id) = cmd.remote_identity_id {
        orchestrator_api_builder.with_trusted_identities(vec![identity_id]);
    }

    let mut api = orchestrator_api_builder
        .as_identity(cmd.cloud_opts.identity.clone())
        .with_endpoint(OrchestratorEndpoint::Controller)
        .with_new_embbeded_node()
        .await?
        .build(&MultiAddr::from_str("/service/version_info")?)
        .await?;

    let resp: VersionInfo = api.request_with_response(Request::get("")).await?;

    println!(
        "{{\"version\":\"{}\",\"project_version\":\"{}\"}}",
        resp.version, resp.project_version
    );
    Ok(())
}
