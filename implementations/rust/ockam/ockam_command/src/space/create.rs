use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use tracing::debug;

use ockam_api::cloud::space::Space;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_api::{Response, Status};
use ockam_core::Route;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{api, connect_to, exitcode, stop_node};
use crate::{CommandGlobalOpts, OutputFormat};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the space.
    #[clap(display_order = 1001)]
    pub name: String,

    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,

    /// Administrators for this space
    #[clap(display_order = 1100, last = true)]
    pub admins: Vec<String>,
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: CreateCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&cmd.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };
        connect_to(port, (opts, cmd), create);
    }
}

async fn create(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route: Route = base_route.modify().append(NODEMANAGER_ADDR).into();
    debug!(?cmd, %route, "Sending request");

    let response: Vec<u8> = ctx
        .send_and_receive(route, api::space::create(cmd)?)
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec
        .decode::<Response>()
        .context("Failed to decode Response")?;
    debug!(?header, "Received response");

    let res = match (header.status(), header.has_body()) {
        (Some(Status::Ok), true) => {
            let body = dec
                .decode::<Space>()
                .context("Failed to decode response body")?;
            let output = match opts.global_args.output_format {
                OutputFormat::Plain => body.id.to_string(),
                OutputFormat::Json => serde_json::to_string(&body)
                    .context("Failed to serialize command output as json")?,
            };
            Ok(output)
        }
        (Some(status), true) => {
            let err = dec
                .decode::<String>()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!(
                "An error occurred while processing the request with status code {status:?}: {err}"
            ))
        }
        _ => Err(anyhow!("Unexpected response received from node")),
    };
    match res {
        Ok(o) => println!("{o}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(exitcode::IOERR);
        }
    };

    stop_node(ctx).await
}
