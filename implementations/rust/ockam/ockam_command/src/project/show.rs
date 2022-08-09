use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use tracing::debug;

use ockam_api::cloud::project::Project;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Response, Status};
use ockam_core::Route;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{api, connect_to, exitcode, stop_node};
use crate::{CommandGlobalOpts, OutputFormat};

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Id of the space.
    /// TODO: this is not used at all?
    #[clap(display_order = 1001)]
    pub space_id: String,

    /// Id of the project.
    #[clap(display_order = 1002)]
    pub project_id: String,
    // TODO: add project_name arg that conflicts with project_id
    //  so we can call the get_project_by_name api method
    // /// Name of the project.
    // #[clap(display_order = 1002)]
    // pub project_name: String,
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ShowCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: ShowCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&cmd.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };
        connect_to(port, (opts, cmd), show);
    }
}

async fn show(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route: Route = base_route.modify().append(NODEMANAGER_ADDR).into();
    debug!(?cmd, %route, "Sending request");

    let response: Vec<u8> = ctx
        .send_and_receive(route, api::project::show(cmd)?)
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => {
            let body = dec.decode::<Project>()?;
            let output = match opts.global_args.output_format {
                OutputFormat::Plain => format!("{body:#?}"),
                OutputFormat::Json => serde_json::to_string(&body)?,
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
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(exitcode::IOERR);
        }
    };

    stop_node(ctx).await
}
