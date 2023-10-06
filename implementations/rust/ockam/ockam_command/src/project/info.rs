use clap::Args;

use ockam::Context;
use ockam_api::cli_state::{ProjectConfigCompact, StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::Projects;

use ockam_api::nodes::InMemoryNode;

use crate::project::util::refresh_projects;
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct InfoCommand {
    /// Name of the project.
    #[arg(default_value = "default")]
    pub name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,

    #[arg(long, default_value = "false")]
    pub as_trust_context: bool,
}

impl InfoCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, InfoCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: InfoCommand) -> miette::Result<()> {
    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    // Lookup project
    let id = match opts.state.projects.get(&cmd.name) {
        Ok(state) => state.config().id.clone(),
        Err(_) => {
            refresh_projects(&opts, ctx, &controller).await?;
            opts.state.projects.get(&cmd.name)?.config().id.clone()
        }
    };

    // Send request

    let project = controller.get_project(ctx, id).await?;
    let info: ProjectConfigCompact = project.into();
    opts.println(&info)?;
    Ok(())
}
