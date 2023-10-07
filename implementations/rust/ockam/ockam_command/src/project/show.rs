use clap::Args;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::Projects;

use ockam_api::nodes::InMemoryNode;

use crate::project::util::refresh_projects;
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show projects
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ShowCommand {
    /// Name of the project.
    #[arg(display_order = 1001)]
    pub name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: ShowCommand) -> miette::Result<()> {
    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    // Lookup project
    let id = match &opts.state.projects.get(&cmd.name) {
        Ok(state) => state.config().id.clone(),
        Err(_) => {
            refresh_projects(&opts, ctx, &controller).await?;
            opts.state.projects.get(&cmd.name)?.config().id.clone()
        }
    };

    // Send request
    let project = controller.get_project(ctx, id).await?;

    opts.println(&project)?;
    opts.state
        .projects
        .overwrite(&project.name, project.clone())?;
    Ok(())
}
