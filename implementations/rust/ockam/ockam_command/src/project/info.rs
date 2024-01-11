use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cloud::project::Projects;

use ockam_api::nodes::InMemoryNode;

use crate::output::{Output, ProjectConfigCompact};
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
        node_rpc(options.rt.clone(), rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, InfoCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: InfoCommand) -> miette::Result<()> {
    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let project = node.get_project_by_name(ctx, &cmd.name).await?;
    let info = ProjectConfigCompact(project);
    opts.terminal
        .stdout()
        .plain(info.output()?)
        .json(serde_json::to_string_pretty(&info).into_diagnostic()?)
        .write_line()?;
    Ok(())
}
