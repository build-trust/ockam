use crate::node::util::default_node_name;
use crate::{help, node::HELP_DETAIL, CommandGlobalOpts};
use clap::Args;

/// Stop a node
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct StopCommand {
    /// Name of the node.
    node_name: Option<String>,
    /// Whether to use the SIGTERM or SIGKILL signal to stop the node
    #[arg(long)]
    force: bool,
}

impl StopCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{}", e);
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: StopCommand) -> crate::Result<()> {
    let node_name = &match cmd.node_name {
        Some(name) => name,
        None => default_node_name(&opts),
    };
    let node_state = opts.state.nodes.get(node_name)?;
    node_state.kill_process(cmd.force)?;
    Ok(())
}
