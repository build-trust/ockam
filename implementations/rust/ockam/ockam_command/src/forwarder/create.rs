use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use serde_json::json;
use tracing::debug;

use ockam_api::nodes::types::ForwarderInfo;
use ockam_api::nodes::NODEMAN_ADDR;
use ockam_api::{Response, Status};
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::{api, connect_to, stop_node, DEFAULT_CLOUD_ADDRESS};
use crate::{CommandGlobalOpts, MessageFormat};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    /// Ockam's cloud address.
    #[clap(default_value = DEFAULT_CLOUD_ADDRESS)]
    addr: MultiAddr,

    /// Forwarder alias. Optional{n}
    /// If set, a static forwarder named after the passed alias will be created{n}
    /// Otherwise, it will create an ephemeral forwarder (default)
    alias: Option<String>,
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: CreateCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&cmd.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };
        connect_to(port, (opts, cmd), create);
    }

    pub fn address(&self) -> &MultiAddr {
        &self.addr
    }

    pub fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }
}

async fn create(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route: Route = base_route.modify().append(NODEMAN_ADDR).into();
    debug!(?cmd, %route, "Sending request");

    let response: Vec<u8> = ctx
        .send_and_receive(route, api::create_forwarder(&cmd)?)
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => {
            let body = dec.decode::<ForwarderInfo>()?;
            let output = match opts.global_args.message_format {
                MessageFormat::Plain => format!(
                    "Forwarder created! You can send messages to it via this address:\n{}",
                    body.remote_address()
                ),
                MessageFormat::Json => json!({
                    "remote_address": body.remote_address(),
                })
                .to_string(),
            };
            Ok(output)
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
