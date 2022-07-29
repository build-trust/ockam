mod create;
mod delete;
mod echoer;
mod list;
mod show;
mod uppercase;

pub(crate) use create::CreateCommand;
use delete::DeleteCommand;
use list::ListCommand;
use show::ShowCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct NodeCommand {
    #[clap(subcommand)]
    subcommand: NodeSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum NodeSubcommand {
    /// Create a node.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Delete a node.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),

    /// List nodes.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),

    /// Show a node.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Show(ShowCommand),
}

impl NodeCommand {
    pub fn run(opts: CommandGlobalOpts, command: NodeCommand) {
        match command.subcommand {
            NodeSubcommand::Create(command) => CreateCommand::run(opts, command),
            NodeSubcommand::Delete(command) => DeleteCommand::run(opts, command),
            NodeSubcommand::List(command) => ListCommand::run(opts, command),
            NodeSubcommand::Show(command) => ShowCommand::run(opts, command),
        }
    }
}

#[derive(Clone, Debug, Args)]
pub struct NodeOpts {
    /// Override the default API node
    #[clap(global = true, name = "node", short, long, default_value = "default")]
    pub api_node: String,
}
