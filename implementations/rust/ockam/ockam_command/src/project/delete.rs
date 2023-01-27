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
    pub project_name: Option<String>,

    /// Id of the project.
    #[arg(display_order = 1003, long, conflicts_with = "project_name")]
    pub project_id: Option<String>,

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
    let space_id = space::config::try_get_space(&opts.config, &cmd.space_name)
        .context(format!("Space '{}' does not exist", cmd.space_name))?;

    let node_name = start_embedded_node(ctx, &opts).await?;
    let controller_route = &cmd.cloud_opts.route();

    let project_id = match (cmd.project_name, cmd.project_id) {
        (Some(project_name), _) => {
            // Lookup project
            let project_id = match config::get_project(&opts.config, &project_name) {
                Some(id) => id,
                None => {
                    // The project is not in the config file.
                    // Fetch all available projects from the cloud.
                    config::refresh_projects(ctx, &opts, &node_name, controller_route, None)
                        .await?;

                    // If the project is not found in the lookup, then it must not exist in the cloud, so we exit the command.
                    match config::get_project(&opts.config, &project_name) {
                        Some(id) => id,
                        None => {
                            return Ok(());
                        }
                    }
                }
            };
            // Try to remove from config again, in case it was re-added after the refresh.
            let _ = config::remove_project(&opts.config, &project_name);
            project_id
        }
        (_, Some(project_id)) => project_id,
        _ => unreachable!("clap should prevent this"),
    };

    // Send request
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::project::delete(
        &space_id,
        &project_id,
        controller_route,
    ))
    .await?;
    rpc.is_ok()?;

    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
