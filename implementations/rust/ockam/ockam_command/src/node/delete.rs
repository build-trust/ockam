use crate::node::default_node_name;
use crate::node::util::{delete_all_nodes, delete_node};
use crate::CommandGlobalOpts;
use clap::Args;
use colorful::Colorful;

/// Delete a node
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct DeleteCommand {
    /// Name of the node.
    #[arg(default_value_t = default_node_name(), group = "nodes")]
    node_name: String,

    /// Terminate all node processes and delete all node configurations
    #[arg(long, short, group = "nodes")]
    all: bool,

    /// Terminate node process(es) immediately (uses SIGKILL instead of SIGTERM)
    #[arg(display_order = 901, long, short)]
    force: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> crate::Result<()> {
    if cmd.all {
        delete_all_nodes(opts, cmd.force)?;
    } else {
        delete_node(&opts, &cmd.node_name, cmd.force)?;
        opts.shell
            .stdout()
            .plain(format!(
                "{}Node with name '{}' has been deleted.",
                "✔︎".light_green(),
                &cmd.node_name
            ))
            .machine(&cmd.node_name)
            .json(&serde_json::json!({ "node": { "name": &cmd.node_name } }))
            .write_line()?;
    }
    Ok(())
}
