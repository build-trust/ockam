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
    /// Delete nodes.
    Delete(DeleteCommand),
    /// List nodes.
    List(ListCommand),
    /// Show nodes.
    Show(ShowCommand),
    /// Start nodes.
    Start(StartCommand),
    /// Spawn nodes.
    Spawn(SpawnCommand),
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
