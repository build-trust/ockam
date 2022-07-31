use crate::node::NodeOpts;
use crate::util::{api, connect_to, stop_node};
use crate::CommandGlobalOpts;
use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use ockam::identity::IdentityIdentifier;
use ockam_api::error::ApiError;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelResponse;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_api::{route_to_multiaddr, Response, Status};
use ockam_core::{route, Route};
use ockam_multiaddr::MultiAddr;
use tracing::debug;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    /// What address to connect to
    addr: MultiAddr,
    /// Pre-known Identifiers of the other side
    #[clap(short, long)]
    authorized_identifier: Option<Vec<IdentityIdentifier>>,
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) -> anyhow::Result<()> {
        let cfg = opts.config;
        let port = match cfg.select_node(&command.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        connect_to(port, command, create_connector);

        Ok(())
    }
}

pub async fn create_connector(
    ctx: ockam::Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let CreateCommand {
        addr,
        authorized_identifier: authorized_identifiers,
        ..
    } = cmd;

    let response: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_secure_channel(addr, authorized_identifiers)?,
        )
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => {
            let body = dec.decode::<CreateSecureChannelResponse>()?;
            let addr = route_to_multiaddr(&route![body.addr.to_string()])
                .ok_or_else(|| ApiError::generic("Invalid Secure Channel Address"))?;
            Ok(format!(
                "Secure Channel created! You can send messages to it via this address:\n{}",
                addr
            ))
        }
        Some(Status::InternalServerError) => {
            let err = dec
                .decode::<String>()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!(
                "An error occurred while processing the request: {err}"
            ))
        }
        _ => Err(anyhow!("Unexpected response received from node")),
    };
    match res {
        Ok(o) => println!("{o}"),
        Err(err) => eprintln!("{err}"),
    };

    stop_node(ctx).await
}
