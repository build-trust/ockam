use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use ockam_api::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
use tracing::debug;

use ockam_api::cloud::project::Project;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Response, Status};
use ockam_core::Route;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{api, connect_to, exitcode, stop_node};
use crate::{CommandGlobalOpts, OutputFormat};
use crate::util::{RpcCaller};

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ListCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: ListCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&cmd.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };
        connect_to(port, (opts, cmd), list);
    }
}

impl<'a> RpcCaller<'a> for ListCommand {
    type Req = BareCloudRequestWrapper<'a>;
    type Resp = Vec<Project<'a>>;

    fn req(&'a self) -> ockam_core::api::RequestBuilder<'_, Self::Req> {
        use ockam_core::api::{Method, Request};

        Request::builder(Method::Get, "v0/projects")
            .body(CloudRequestWrapper::bare(self.cloud_opts.route()))
    }
}

async fn list(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route: Route = base_route.modify().append(NODEMANAGER_ADDR).into();
    debug!(?cmd, %route, "Sending request");

    let response: Vec<u8> = ctx
        .send_and_receive(route, api::project::list(&cmd).to_vec()?)
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => {
            let body = dec.decode::<Vec<Project>>()?;
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
