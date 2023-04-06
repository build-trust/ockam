use clap::Args;

use std::str::FromStr;
use std::time::Duration;

use anyhow::anyhow;

use ockam_core::api::{Request, Response, Status};
use ockam_core::CowStr;

use minicbor::{Decode, Decoder, Encode};

use crate::util::node_rpc;
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
    /// Node tcp address
    #[arg(long, required = true, value_name = "TCP_ADDRESS")]
    pub tcp_address: String,
    /// Node identity id
    #[arg(long, value_name = "REMOTE_IDENTITY_ID")]
    pub remote_identity_id: Option<String>,
    /// Override Default Timeout
    #[arg(long, value_name = "TIMEOUT", default_value = "10000")]
    pub timeout: u64,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(get_orchestrator_version, (options, self))
    }
}

async fn get_orchestrator_version(
    ctx: Context,
    (_opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    let connection = tcp
        .connect(cmd.tcp_address, TcpConnectionOptions::new())
        .await?;

    let node = node(ctx);
    let identity = node.create_identity().await?;

    let r = route![connection, "api"];

    let secure_channel_timeout = Duration::from_millis(50_000);

    let channel = match cmd.remote_identity_id {
        Some(identity_id) => {
            node.create_secure_channel_extended(
                &identity,
                r,
                SecureChannelOptions::new().with_trust_policy(TrustIdentifierPolicy::new(
                    IdentityIdentifier::from_str(&identity_id)?,
                )),
                secure_channel_timeout,
            )
            .await?
        }
        None => {
            node.create_secure_channel_extended(
                &identity,
                r,
                SecureChannelOptions::new().with_trust_policy(TrustEveryonePolicy),
                secure_channel_timeout,
            )
            .await?
        }
    };

    let req = Request::get("");

    let buf: Vec<u8> = node
        .send_and_receive_extended::<Vec<u8>>(
            route![channel, "version_info"],
            req.to_vec()?,
            MessageSendReceiveOptions::new().with_timeout(Duration::from_millis(cmd.timeout)),
        )
        .await?
        .body();

    let mut dec = Decoder::new(&buf);

    let hdr = dec.decode::<Response>()?;

    if hdr.status() == Some(Status::Ok) {
        let node_info = dec.decode::<VersionInfo>()?;
        println!(
            "{{\"version\":\"{}\",\"project_version\":\"{}\"}}",
            node_info.version, node_info.project_version
        );
        Ok(())
    } else {
        Err(anyhow!("Request status not 200: {:?}", hdr.status()).into())
    }
}
