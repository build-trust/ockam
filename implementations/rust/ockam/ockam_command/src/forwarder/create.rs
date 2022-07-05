use anyhow::Context;
use clap::Args;
use minicbor::Decoder;
use tracing::debug;

use ockam_api::nodes::types::ForwarderInfo;
use ockam_api::{Response, Status};
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;

use crate::util::{api, connect_to, stop_node};
use crate::OckamConfig;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Ockam's cloud node address.
    address: MultiAddr,

    /// Forwarder alias. Optional{n}
    /// If set, a static forwarder named after the passed alias will be created{n}
    /// Otherwise, it will create an ephemeral forwarder (default)
    alias: Option<String>,

    /// The API node name to communicate with.
    #[clap(short, long)]
    node_name: Option<String>,
}

impl CreateCommand {
    pub fn run(cfg: &OckamConfig, cmd: CreateCommand) {
        let port = match cfg.select_node(&cmd.node_name) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };
        connect_to(port, cmd, create);
    }

    pub fn address(&self) -> &MultiAddr {
        &self.address
    }

    pub fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }
}

async fn create(
    ctx: ockam::Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route: Route = base_route.modify().append("_internal.nodeman").into();
    debug!(?cmd, %route, "Sending request to create forwarder");
    let response: Vec<u8> = ctx
        .send_and_receive(route, api::create_forwarder(&cmd)?)
        .await
        .context("failed to create forwarder")?;
    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received CreateForwarder response");
    match header.status() {
        Some(Status::Ok) => {
            let body = dec.decode::<ForwarderInfo>()?;
            println!(
                "Forwarder created! You can send messages to it via this address:\n{}",
                body.remote_address()
            );
        }
        Some(Status::InternalServerError) => {
            let err = dec.decode::<String>()?;
            eprintln!("An error occurred while creating the forwarder: {err}")
        }
        _ => {
            eprintln!("Unexpected response received from node")
        }
    }

    stop_node(ctx).await
}
