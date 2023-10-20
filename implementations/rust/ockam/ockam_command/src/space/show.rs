use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cli_state::{SpaceConfig, StateDirTrait, StateItemTrait};
use ockam_api::cloud::space::{Space, Spaces};
use ockam_api::nodes::InMemoryNode;

use crate::output::Output;
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of a space
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name of the space.
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
    let space = get_space(ctx, &opts, &node, &cmd.name).await?;
    display_space(opts, &space)?;
    Ok(())
}

async fn get_space(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node: &InMemoryNode,
    space_name: &str,
) -> miette::Result<Space> {
    // Send request
    let id = opts.state.spaces.get(&space_name)?.config().id.clone();

    let controller = node.create_controller().await?;

    controller.get_space(ctx, id).await
}

fn display_space(opts: CommandGlobalOpts, space: &Space) -> Result<(), miette::ErrReport> {
    opts.terminal
        .stdout()
        .plain(space.output()?)
        .json(serde_json::to_string_pretty(space).into_diagnostic()?)
        .write_line()?;
    opts.state
        .spaces
        .overwrite(&space.name, SpaceConfig::from(space))?;
    Ok(())
}
