use crate::node::util::{delete_all_nodes, delete_node};
use crate::node::{default_node_name, node_name_parser};
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
    #[arg(default_value_t = default_node_name(), value_parser = node_name_parser, group = "nodes")]
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
        let state = &opts.state.nodes;
        match state.get(&cmd.node_name) {
            // If it exists, proceed
            Ok(_) => {
                delete_node(&opts, &cmd.node_name, cmd.force)?;
                opts.terminal
                    .stdout()
                    .plain(format!(
                        "{} Node with name '{}' has been deleted.",
                        "✔︎".light_green(),
                        &cmd.node_name
                    ))
                    .machine(&cmd.node_name)
                    .json(serde_json::json!({ "node": { "name": &cmd.node_name } }))
                    .write_line()?;
            }
            // Else, return the appropriate error
            Err(err) => match err {
                CliStateError::NotFound => {
                    return Err(anyhow!("Node '{}' not found", &cmd.node_name).into())
                }
                _ => return Err(err.into()),
            },
        }
    }
    Ok(())
}
