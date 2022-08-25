use clap::Args;

use ockam_api::cloud::project::Project;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{stop_node, RpcAlt, RpcCaller, node_rpc};
use crate::{CommandGlobalOpts};
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
}

impl<'a> RpcCaller<'a> for CreateCommand {
    type Req = CloudRequestWrapper<'a, CreateProject<'a>>;
    type Resp = Project<'a>;

    fn req(&'a self) -> ockam_core::api::RequestBuilder<'a, Self::Req> {
        use ockam_core::api::{Method, Request};

        let b = CreateProject::new(self.project_name.as_str(), &[], &self.services);
        Request::builder(Method::Post, format!("v0/projects/{}", self.space_id))
            .body(CloudRequestWrapper::new(b, self.cloud_opts.route()))
    }
}

async fn rpc(ctx: ockam::Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> crate::Result<()> {
    let res = rpc_callback(cmd, &ctx, opts).await;
    stop_node(ctx).await?;
    res
}

async fn rpc_callback(mut cmd: CreateCommand, ctx: &ockam::Context, opts: CommandGlobalOpts) -> crate::Result<()> {
    // We apply the inverse transformation done in the `create` command.

    let node = cmd.node_opts.api_node.clone();
    RpcAlt::new(ctx, &opts, &node)?
        .request_then_response(&mut cmd).await?.parse_body()?.print(&opts)
}
/*
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
*/
