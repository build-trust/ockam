use clap::Args;
use colorful::Colorful;

use ockam_node::Context;

use crate::fmt_ok;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/logs/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/logs/after_long_help.txt");

/// Get the stdout/stderr log file of a node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct LogCommand {
    /// Name of the node to retrieve the logs from.
    node_name: Option<String>,
}

impl LogCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, LogCommand),
) -> miette::Result<()> {
    let node_name = opts.state.get_node_or_default(&cmd.node_name).await?.name();
    let log_path = opts.state.stdout_logs(&node_name)?.display().to_string();
    opts.terminal
        .stdout()
        .plain(fmt_ok!("The path for the log file is: {log_path}"))
        .machine(&log_path)
        .json(serde_json::json!({ "path": log_path }))
        .write_line()?;
    Ok(())
}
