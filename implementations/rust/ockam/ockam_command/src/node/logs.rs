use crate::node::default_node_name;
use crate::{help, node::HELP_DETAIL, CommandGlobalOpts};
use clap::Args;
use std::path::PathBuf;

/// Get the stdout/stderr log file of a node
#[derive(Clone, Debug, Args)]
#[command(
    after_long_help = help::template(HELP_DETAIL)
)]
pub struct LogCommand {
    /// Name of the node.
    #[arg(default_value_t = default_node_name())]
    node_name: String,

    /// Show the standard error log file.
    #[arg(long = "err")]
    show_err: bool,
}

impl LogCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: LogCommand) -> crate::Result<PathBuf> {
    let node_state = opts.state.nodes.get(&cmd.node_name)?;

    let log_file_path = if cmd.show_err {
        node_state.stderr_log()
    } else {
        node_state.stdout_log()
    };
    println!("{}", log_file_path.display());
    Ok(log_file_path)
}
