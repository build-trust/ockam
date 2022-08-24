use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use tracing::debug;

use ockam_api::cloud::project::Project;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Method, Request, Response, Status};
use ockam_core::Route;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{api, connect_to, exitcode, stop_node, Rpc1, CmdTrait, node_rpc};
use crate::{CommandGlobalOpts, OutputFormat};
use ockam_api::cloud::{CloudRequestWrapper};
use ockam_api::cloud::project::{CreateProject};


#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Id of the space the project belongs to.
    #[clap(display_order = 1001)]
    pub space_id: String,

    /// Name of the project.
    #[clap(display_order = 1002)]
    pub project_name: String,

    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,

    /// Services enabled for this project.
    #[clap(display_order = 1100, last = true)]
    pub services: Vec<String>,
    //TODO:  list of admins
    #[clap(skip)]
    pub global_opts: Option<CommandGlobalOpts>,
}

impl CreateCommand {
    pub fn run(mut self, opts: CommandGlobalOpts) {
        self.global_opts = Some(opts.clone());
        node_rpc(rpc, (opts, self));
    }

/*    
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
    */
}

impl<'a> CmdTrait<'a> for CreateCommand {
    type Req = CloudRequestWrapper<'a, CreateProject<'a>>;
    type Resp = Project<'a>;

    fn req(&'a mut self) -> ockam_core::api::RequestBuilder<'a, Self::Req> {
        api::project::create(self)
/*        let project_name = self.project_name.clone().as_str();
        let services = self.services.clone();
        let b = CreateProject::new(project_name, &[], &services);
        let req = Request::builder(Method::Post, format!("v0/projects/{}", self.space_id))
            .body(CloudRequestWrapper::new(b, self.cloud_opts.route()));
        req  
        */  
    }
}

async fn rpc(ctx: ockam::Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> crate::Result<()> {
    let res = rpc_callback(cmd, &ctx, opts).await;
    stop_node(ctx).await?;
    res
}

async fn rpc_callback(cmd: CreateCommand, ctx: &ockam::Context, opts: CommandGlobalOpts) -> crate::Result<()> {
    // We apply the inverse transformation done in the `create` command.
/*
    let from = cmd.node_opts.from.clone();
    let mut rpc = Rpc1::new(ctx, cmd, &opts, &from)?;
    let res = rpc.request_then_response().await?;

    let mut dec = Decoder::new(&res);
    let res: <CreateCommand as CmdTrait>::Resp = dec.decode().context("Failed to decode response body")?;
    res
    */
    Ok(())
}

async fn create(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route: Route = base_route.modify().append(NODEMANAGER_ADDR).into();
    debug!(?cmd, %route, "Sending request");

    let response: Vec<u8> = ctx
        .send_and_receive(route, api::project::create(&cmd).to_vec()?)
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => {
            let body = dec.decode::<Project>()?;
            let output = match opts.global_args.output_format {
                OutputFormat::Plain => body.id.to_string(),
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
