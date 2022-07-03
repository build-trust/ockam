mod create;
mod delete;
mod list;
mod purge;
mod show;

pub(crate) use create::CreateCommand;
use delete::DeleteCommand;
use list::ListCommand;
use purge::PurgeCommand;
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

    /// Purge all node configuration (great for development)
    #[clap(display_order = 1005, hide = true)]
    Purge(PurgeCommand),
}

impl NodeCommand {
    pub fn run(opts: CommandGlobalOpts, command: NodeCommand) {
        match command.subcommand {
            NodeSubcommand::Create(command) => CreateCommand::run(opts, command),
            NodeSubcommand::Delete(command) => DeleteCommand::run(opts, command),
            NodeSubcommand::List(command) => ListCommand::run(opts, command),
            NodeSubcommand::Show(command) => ShowCommand::run(opts, command),
            NodeSubcommand::Purge(command) => PurgeCommand::run(opts, command),
        }
    }
}
