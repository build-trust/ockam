use crate::node::get_node_name;
use crate::node::util::{delete_all_nodes, delete_node};
use crate::{docs, CommandGlobalOpts};
use anyhow::anyhow;
use clap::Args;
use colorful::Colorful;
use ockam_api::cli_state::{CliStateError, StateDirTrait};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete nodes
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the node.
    #[arg(group = "nodes")]
    node_name: Option<String>,

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
        let state = &opts.state.nodes;
        let node_name = get_node_name(&opts.state, cmd.node_name.clone())?;
        match state.get(&node_name) {
            // If it exists, proceed
            Ok(_) => {
                delete_node(&opts, &node_name, cmd.force)?;
                opts.terminal
                    .stdout()
                    .plain(format!(
                        "{} Node with name '{}' has been deleted.",
                        "✔︎".light_green(),
                        &node_name
                    ))
                    .machine(&node_name)
                    .json(serde_json::json!({ "node": { "name": &node_name } }))
                    .write_line()?;
            }
            // Else, return the appropriate error
            Err(err) => match err {
                CliStateError::NotFound => {
                    return Err(anyhow!("Node '{}' not found", &node_name).into())
                }
                _ => return Err(err.into()),
            },
        }
    }
    Ok(())
}
