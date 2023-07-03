use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::util::refresh_projects;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete projects
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
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
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> miette::Result<()> {
    let space_id = opts.state.spaces.get(&cmd.space_name)?.config().id.clone();

    let node_name = start_embedded_node(ctx, &opts, None).await?;
    let controller_route = &CloudOpts::route();

    // Try to remove from config, in case the project was removed from the cloud but not from the config file.
    opts.state.projects.delete(&cmd.project_name)?;

    // Lookup project
    let project_id = match opts.state.projects.get(&cmd.project_name) {
        Ok(ref state) => state.config().id.clone(),
        Err(_) => {
            // The project is not in the config file.
            // Fetch all available projects from the cloud.
            refresh_projects(ctx, &opts, &node_name, controller_route, None).await?;

            // If the project is not found in the lookup, then it must not exist in the cloud, so we exit the command.
            match opts.state.projects.get(&cmd.project_name) {
                Ok(ref state) => state.config().id.clone(),
                Err(_) => {
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
    opts.state.projects.delete(&cmd.project_name)?;

    delete_embedded_node(&opts, rpc.node_name()).await;

    // log the deletion
    opts.terminal
        .stdout()
        .plain(fmt_ok!(
            "Project with name '{}' has been deleted.",
            &cmd.project_name
        ))
        .machine(&cmd.project_name)
        .json(serde_json::json!({ "project": { "name": &cmd.project_name } }))
        .write_line()?;

    Ok(())
}
