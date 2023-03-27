use crate::node::default_node_name;
use crate::{docs, CommandGlobalOpts};
use clap::Args;

const LONG_ABOUT: &str = include_str!("./static/stop/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/stop/after_long_help.txt");

/// Stop a running node
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct StopCommand {
    /// Name of the node.
    #[arg(default_value_t = default_node_name())]
    node_name: String,
    /// Whether to use the SIGTERM or SIGKILL signal to stop the node
    #[arg(long)]
    force: bool,
}

impl StopCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: StopCommand) -> crate::Result<()> {
    let node_state = opts.state.nodes.get(&cmd.node_name)?;
    node_state.kill_process(cmd.force)?;
    println!("Stopped node '{}'", &cmd.node_name);
    Ok(())
}
