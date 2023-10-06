use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::Projects;

use ockam_api::nodes::InMemoryNode;

use crate::project::util::refresh_projects;
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
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
    /// Name of the space
    #[arg(display_order = 1001)]
    pub space_name: String,

    /// Name of the project
    #[arg(display_order = 1002)]
    pub project_name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> miette::Result<()> {
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(cmd.yes, "Are you sure you want to delete this project?")?
    {
        let space_id = opts.state.spaces.get(&cmd.space_name)?.config().id.clone();
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        // Lookup project
        let project_id = match opts.state.projects.get(&cmd.project_name) {
            Ok(state) => state.config().id.clone(),
            Err(_) => {
                // The project is not in the config file.
                // Fetch all available projects from the cloud.
                refresh_projects(&opts, ctx, &controller).await?;

                // If the project is not found in the lookup, then it must not exist in the cloud, so we exit the command.
                match opts.state.projects.get(&cmd.project_name) {
                    Ok(state) => state.config().id.clone(),
                    Err(_) => {
                        return Ok(());
                    }
                }
            }
        };

        // Send request
        controller.delete_project(ctx, space_id, project_id).await?;

        opts.state.projects.delete(&cmd.project_name)?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Project with name '{}' has been deleted.",
                &cmd.project_name
            ))
            .machine(&cmd.project_name)
            .json(serde_json::json!({ "project": { "name": &cmd.project_name } }))
            .write_line()?;
    }
    Ok(())
}
