use anyhow::Context as _;
use clap::Args;
use rand::prelude::random;

use ockam::Context;
use ockam_api::cloud::project::Project;

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::util::{check_project_readiness, config};
use crate::util::api::CloudOpts;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::{space, CommandGlobalOpts};

/// Create projects
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the space the project belongs to.
    #[clap(display_order = 1001)]
    pub space_name: String,

    /// Name of the project.
    #[clap(display_order = 1002, default_value_t = hex::encode(&random::<[u8;4]>()), hide_default_value = true)]
    pub project_name: String,

    // Enforce credentials for member access to the project node
    #[clap(long, display_order = 1003)]
    pub enforce_credentials: Option<bool>,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,

    /// Services enabled for this project.
    #[clap(display_order = 1100, last = true)]
    pub services: Vec<String>,
    //TODO:  list of admins
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
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
    let space_id = space::config::try_get_space(&opts.config, &cmd.space_name)
        .context(format!("Space '{}' does not exist", cmd.space_name))?;
    let node_name = start_embedded_node(ctx, &opts.config).await?;
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::project::create(
        &cmd.project_name,
        &space_id,
        cmd.enforce_credentials,
        &cmd.cloud_opts.route(),
    ))
    .await?;
    let project = rpc.parse_response::<Project>()?;
    let project =
        check_project_readiness(ctx, &opts, &cmd.cloud_opts, &node_name, None, project).await?;
    config::set_project(&opts.config, &project).await?;
    rpc.print_response(project)?;
    delete_embedded_node(&opts.config, rpc.node_name()).await;
    Ok(())
}
