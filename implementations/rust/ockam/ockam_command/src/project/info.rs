use clap::Args;

use ockam::Context;
use ockam_api::cli_state::{ProjectConfigCompact, StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::Project;

use crate::node::util::delete_embedded_node;
use crate::project::util::refresh_projects;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
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

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, InfoCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: InfoCommand,
) -> miette::Result<()> {
    let mut rpc = Rpc::embedded(ctx, &opts).await?;

    // Lookup project
    let id = match opts.state.projects.get(&cmd.name) {
        Ok(state) => state.config().id.clone(),
        Err(_) => {
            refresh_projects(&opts, &mut rpc).await?;
            opts.state.projects.get(&cmd.name)?.config().id.clone()
        }
    };

    // Send request
    let project: Project = rpc.ask(api::project::show(&id)).await?;
    let info: ProjectConfigCompact = project.into();
    opts.println(&info)?;

    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
