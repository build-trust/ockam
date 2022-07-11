use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use serde_json::json;
use tracing::debug;

use ockam_api::{Response, Status};
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::{api, connect_to, stop_node, DEFAULT_CLOUD_ADDRESS};
use crate::{CommandGlobalOpts, MessageFormat};

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Id of the space.
    #[clap(display_order = 1001)]
    pub id: String,

    #[clap(flatten)]
    node_opts: NodeOpts,

    /// Ockam's cloud address. Argument used for testing purposes.
    #[clap(hide = true, display_order = 1100, default_value = DEFAULT_CLOUD_ADDRESS)]
    addr: MultiAddr,
}

impl DeleteCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: DeleteCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&cmd.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };
        connect_to(port, (opts, cmd), delete);
    }
}

async fn delete(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route: Route = base_route.modify().append("_internal.nodeman").into();
    debug!(?cmd, %route, "Sending request");

    let response: Vec<u8> = ctx
        .send_and_receive(route, api::space::delete(&cmd)?)
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => {
            let output = match opts.global_args.message_format {
                MessageFormat::Plain => "Space deleted".to_string(),
                MessageFormat::Json => json!({
                    "id": cmd.id,
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
