mod create;
mod delete;
mod list;
mod purge;
mod show;

use create::CreateCommand;
use delete::DeleteCommand;
use list::ListCommand;
use purge::PurgeCommand;
use show::ShowCommand;

use crate::{util::OckamConfig, HELP_TEMPLATE};
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
    pub fn run(cfg: &mut OckamConfig, command: NodeCommand) {
        match command.subcommand {
            NodeSubcommand::Create(command) => CreateCommand::run(cfg, command),
            NodeSubcommand::Delete(command) => DeleteCommand::run(cfg, command),
            NodeSubcommand::List(command) => ListCommand::run(cfg, command),
            NodeSubcommand::Show(command) => ShowCommand::run(cfg, command),
            NodeSubcommand::Purge(command) => PurgeCommand::run(cfg, command),
        }
    }
}
