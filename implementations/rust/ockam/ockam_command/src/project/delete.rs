use clap::Args;
use colorful::Colorful;

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
    let space_id = space::config::try_get_space(&opts.config, &cmd.space_name)?;

    let node_name = start_embedded_node(ctx, &opts, None).await?;
    let controller_route = &cmd.cloud_opts.route();

    // Try to remove from config, in case the project was removed from the cloud but not from the config file.
    let _ = config::remove_project(&opts.config, &cmd.project_name);

    // Lookup project
    let project_id = match config::get_project(&opts.config, &cmd.project_name) {
        Some(id) => id,
        None => {
            // The project is not in the config file.
            // Fetch all available projects from the cloud.
            config::refresh_projects(ctx, &opts, &node_name, controller_route, None).await?;

            // If the project is not found in the lookup, then it must not exist in the cloud, so we exit the command.
            match config::get_project(&opts.config, &cmd.project_name) {
                Some(id) => id,
                None => {
                    return Ok(());
                }
            }
        }
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

    // Try to remove from config again, in case it was re-added after the refresh.
    let _ = config::remove_project(&opts.config, &cmd.project_name);

    delete_embedded_node(&opts, rpc.node_name()).await;

    // log the deletion
    opts.shell
        .stdout()
        .plain(format!(
            "{}Project with name '{}' has been deleted.",
            "✔︎".light_green(),
            &cmd.project_name
        ))
        .machine(&cmd.project_name)
        .json(&serde_json::json!({ "project": { "name": &cmd.project_name } }))
        .write_line()?;

    Ok(())
}
