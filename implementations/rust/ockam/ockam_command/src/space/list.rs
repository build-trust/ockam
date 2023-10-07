use clap::Args;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::cli_state::{SpaceConfig, StateDirTrait};
use ockam_api::cloud::space::Spaces;

use ockam_api::nodes::InMemoryNode;

use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List spaces
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, _cmd: ListCommand) -> miette::Result<()> {
    let is_finished: Mutex<bool> = Mutex::new(false);
    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    let get_spaces = async {
        let spaces = controller.list_spaces(ctx).await?;
        *is_finished.lock().await = true;
        Ok(spaces)
    };

    let output_messages = vec![format!("Listing Spaces...\n",)];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (spaces, _) = try_join!(get_spaces, progress_output)?;

    let plain = opts.terminal.build_list(
        &spaces,
        "Spaces",
        "No spaces found. Run 'ockam enroll' to get a space and a project",
    )?;
    let json = serde_json::to_string_pretty(&spaces).into_diagnostic()?;

    for space in spaces {
        opts.state
            .spaces
            .overwrite(&space.name, SpaceConfig::from(&space))?;
    }

    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;
    Ok(())
}
