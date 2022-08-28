use anyhow::Context as _;
use clap::Args;
use ockam::{Context, TcpTransport};

use ockam_api::cloud::project::Project;

use crate::node::NodeOpts;
use crate::project::util::{check_project_readiness, config};
use crate::util::api::CloudOpts;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::{space, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the space the project belongs to.
    #[clap(display_order = 1001)]
    pub space_name: String,

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
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: CreateCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: CreateCommand,
) -> crate::Result<()> {
    let space_id = space::config::get_space(&opts.config, &cmd.space_name)
        .context(format!("Space '{}' does not exist", cmd.space_name))?;
    let tcp = TcpTransport::create(ctx).await?;
    let mut rpc = RpcBuilder::new(ctx, &opts, &cmd.node_opts.api_node)
        .tcp(&tcp)
        .build()?;
    rpc.request(api::project::create(
        &cmd.project_name,
        &space_id,
        cmd.cloud_opts.route(),
    ))
    .await?;
    let project = rpc.parse_response::<Project>()?;
    let project =
        check_project_readiness(ctx, &opts, &cmd.node_opts, &cmd.cloud_opts, &tcp, project).await?;
    config::set_project(&opts.config, &project)?;
    rpc.print_response(project)?;
    Ok(())
}
