use crate::node::default_node_name;
use crate::CommandGlobalOpts;
use clap::Args;

/// Stop a node
#[derive(Clone, Debug, Args)]
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
