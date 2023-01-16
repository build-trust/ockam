use anyhow::Context as _;
use clap::Args;

use ockam::Context;

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::util::config;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::{space, CommandGlobalOpts};

/// Delete projects
#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Name of the space.
    #[arg(display_order = 1001)]
    pub space_name: String,

    /// Name of the project.
    #[arg(display_order = 1002)]
    pub project_name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> crate::Result<()> {
    let node_name = start_embedded_node(ctx, &opts).await?;
    let node_state = opts.state.nodes.get(&node_name)?;

    let space_id = space::config::try_get_space(&node_state, &cmd.space_name)
        .context(format!("Space '{}' does not exist", cmd.space_name))?;

    let controller_route = &cmd.cloud_opts.route();

    // Try to remove from config, in case the project was removed from the cloud but not from the config file.
    let _ = config::remove_project(&node_state, &cmd.project_name);

    // Lookup project
    let project_id = config::get_project(
        ctx,
        &opts,
        &cmd.project_name,
        &node_name,
        controller_route,
        None,
    )
    .await?;

    // Send request
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::project::delete(
        &space_id,
        &project_id,
        controller_route,
    ))
    .await?;
    rpc.is_ok()?;

    // Try to remove from config again, in case it was re-added after the refresh.
    let _ = config::remove_project(&node_state, &cmd.project_name);

    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
