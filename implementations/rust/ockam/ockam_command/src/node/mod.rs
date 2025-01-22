use clap::{Args, Subcommand};

pub use create::CreateCommand;
pub use create::*;
use default::DefaultCommand;
use delete::DeleteCommand;
use list::ListCommand;
use logs::LogCommand;
use ockam_api::address::extract_address_value;
use show::ShowCommand;
use start::StartCommand;
use stop::StopCommand;

use crate::{docs, Command, CommandGlobalOpts};

mod create;
mod default;
mod delete;
mod list;
mod logs;
pub(crate) mod show;
mod start;
mod stop;
pub mod util;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Manage Nodes
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct NodeCommand {
    #[command(subcommand)]
    pub subcommand: NodeSubcommand,
}

impl NodeCommand {
    pub fn name(&self) -> String {
        self.subcommand.name()
    }
}

#[derive(Clone, Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum NodeSubcommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
    Logs(LogCommand),
    Show(ShowCommand),
    Start(StartCommand),
    Stop(StopCommand),
    Default(DefaultCommand),
}

impl NodeSubcommand {
    pub fn name(&self) -> String {
        match self {
            NodeSubcommand::Create(c) => c.name(),
            NodeSubcommand::Delete(c) => c.name(),
            NodeSubcommand::List(c) => c.name(),
            NodeSubcommand::Logs(c) => c.name(),
            NodeSubcommand::Show(c) => c.name(),
            NodeSubcommand::Start(c) => c.name(),
            NodeSubcommand::Stop(c) => c.name(),
            NodeSubcommand::Default(c) => c.name(),
        }
    }
}

impl NodeCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            NodeSubcommand::Create(c) => c.run(opts),
            NodeSubcommand::Delete(c) => c.run(opts),
            NodeSubcommand::List(c) => c.run(opts),
            NodeSubcommand::Show(c) => c.run(opts),
            NodeSubcommand::Start(c) => c.run(opts),
            NodeSubcommand::Stop(c) => c.run(opts),
            NodeSubcommand::Logs(c) => c.run(opts),
            NodeSubcommand::Default(c) => c.run(opts),
        }
    }
}

#[derive(Clone, Debug, Args)]
pub struct NodeOpts {
    /// Perform the command on the given node
    #[arg(global = true, id = "at", value_name = "NODE_NAME", long, value_parser = extract_address_value)]
    pub at_node: Option<String>,
}
