use crate::node::get_node_name;
use crate::util::local_cmd;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use ockam_api::cli_state::StateDirTrait;

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

    /// Show the standard error log file.
    #[arg(long = "err")]
    show_err: bool,
}

impl LogCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: LogCommand) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_name);
    let node_state = opts.state.nodes.get(node_name)?;
    let log_file_path = if cmd.show_err {
        node_state.stderr_log()
    } else {
        node_state.stdout_log()
    };
    opts.terminal
        .stdout()
        .machine(log_file_path.display().to_string())
        .write_line()?;
    Ok(())
}
