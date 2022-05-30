mod delete;
mod list;
mod show;
mod spawn;
mod start;

use delete::DeleteCommand;
use list::ListCommand;
use show::ShowCommand;
use spawn::SpawnCommand;
use start::StartCommand;

use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct NodeCommand {
    #[clap(subcommand)]
    subcommand: NodeSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum NodeSubcommand {
    /// List nodes.
    #[clap(display_order = 900)]
    List(ListCommand),

    /// Show nodes.
    #[clap(display_order = 901)]
    Show(ShowCommand),

    /// Start nodes.
    #[clap(display_order = 902)]
    Start(StartCommand),

    /// Spawn nodes.
    #[clap(display_order = 903)]
    Spawn(SpawnCommand),

    /// Delete nodes.
    #[clap(display_order = 904)]
    Delete(DeleteCommand),
}

impl NodeCommand {
    pub fn run(command: NodeCommand) {
        match command.subcommand {
            NodeSubcommand::Start(command) => StartCommand::run(command),
            NodeSubcommand::Spawn(command) => SpawnCommand::run(command),
            NodeSubcommand::Delete(command) => DeleteCommand::run(command),
            NodeSubcommand::List(command) => ListCommand::run(command),
            NodeSubcommand::Show(command) => ShowCommand::run(command),
        }
    }
}
