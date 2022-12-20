use crate::node::util::{default_node_name, delete_all_nodes, delete_node};
use crate::{help, node::HELP_DETAIL, CommandGlobalOpts};
use clap::Args;

/// Delete a node
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct DeleteCommand {
    /// Name of the node.
    #[arg(group = "nodes")]
    node_name: Option<String>,

    /// Terminate all nodes
    #[arg(long, short, group = "nodes")]
    all: bool,

    /// Clean up config directories and all nodes state directories
    #[arg(display_order = 901, long, short)]
    force: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{}", e);
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> crate::Result<()> {
    if cmd.all {
        delete_all_nodes(opts, cmd.force)?;
    } else {
        let node_name = &match cmd.node_name {
            Some(name) => name,
            None => default_node_name(&opts),
        };
        delete_node(&opts, node_name, cmd.force)?;
        println!("Deleted node '{}'", node_name);
    }
    Ok(())
}
