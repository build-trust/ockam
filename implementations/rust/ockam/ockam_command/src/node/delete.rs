use crate::node::util::{delete_all_nodes, delete_node};
use crate::{help, node::HELP_DETAIL, CommandGlobalOpts};
use clap::Args;

/// Delete Nodes
#[derive(Clone, Debug, Args)]
#[clap(arg_required_else_help = true, help_template = help::template(HELP_DETAIL))]
pub struct DeleteCommand {
    /// Name of the node.
    #[clap(default_value = "default", hide_default_value = true, group = "nodes")]
    node_name: String,

    /// Terminate all nodes
    #[clap(long, short, group = "nodes")]
    all: bool,

    /// Clean up config directories and all nodes state directories
    #[clap(display_order = 901, long, short)]
    force: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(options, self) {
            eprintln!("{}", e);
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> crate::Result<()> {
    if cmd.all {
        delete_all_nodes(opts, cmd.force)?;
    } else {
        delete_node(&opts, &cmd.node_name, cmd.force);
        opts.config.persist_config_updates()?;
        println!("Deleted node '{}'", &cmd.node_name);
    }
    Ok(())
}
