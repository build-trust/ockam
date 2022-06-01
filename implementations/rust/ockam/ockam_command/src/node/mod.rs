mod create;
mod delete;
mod list;
mod show;

use create::CreateCommand;
use delete::DeleteCommand;
use list::ListCommand;
use show::ShowCommand;

use crate::{config::OckamConfig, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct NodeCommand {
    #[clap(subcommand)]
    subcommand: NodeSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum NodeSubcommand {
    /// Create nodes.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Delete nodes.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),

    /// List nodes.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),

    /// Show nodes.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Show(ShowCommand),
}

impl NodeCommand {
    pub fn run(cfg: &mut OckamConfig, command: NodeCommand) {
        match command.subcommand {
            NodeSubcommand::Create(command) => CreateCommand::run(cfg, command),
            NodeSubcommand::Delete(command) => DeleteCommand::run(cfg, command),
            NodeSubcommand::List(command) => ListCommand::run(cfg, command),
            NodeSubcommand::Show(command) => ShowCommand::run(cfg, command),
        }
    }
}
