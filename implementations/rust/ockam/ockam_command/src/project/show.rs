use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cloud::project::Projects;
use ockam_api::nodes::InMemoryNode;

use crate::output::output::Output;
use crate::output::ProjectConfigCompact;
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
    let project = node.get_project_by_name(ctx, &cmd.name).await?;
    let project_output = ProjectConfigCompact(project);

    opts.terminal
        .stdout()
        .plain(project_output.output()?)
        .json(serde_json::to_string_pretty(&project_output).into_diagnostic()?)
        .write_line()?;
    Ok(())
}
