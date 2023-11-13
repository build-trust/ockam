use clap::Args;
use colorful::Colorful;

use ockam_node::Context;

use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/stop/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/stop/after_long_help.txt");

/// Stop a running node
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct StopCommand {
    /// Name of the node.
    node_name: Option<String>,
    /// Whether to use the SIGTERM or SIGKILL signal to stop the node
    #[arg(short, long)]
    force: bool,
}

impl StopCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, StopCommand),
) -> miette::Result<()> {
    let node_name = opts.state.get_node_name_or_default(&cmd.node_name).await?;
    opts.state.stop_node(&node_name, cmd.force).await?;
    opts.terminal
        .stdout()
        .plain(fmt_ok!("Stopped node '{}'", &node_name))
        .write_line()?;
    Ok(())
}
