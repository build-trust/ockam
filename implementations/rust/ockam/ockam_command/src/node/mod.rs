use clap::{Args, Subcommand};

pub(crate) use create::CreateCommand;
use default::DefaultCommand;
use delete::DeleteCommand;
use list::ListCommand;
use logs::LogCommand;
use ockam_api::cli_state::CliState;
use show::ShowCommand;
use start::StartCommand;
use stop::StopCommand;

use crate::{help, CommandGlobalOpts};

mod create;
mod default;
mod delete;
mod list;
mod logs;
mod show;
mod start;
mod stop;
pub mod util;
pub use create::*;

const HELP_DETAIL: &str = include_str!("../constants/node/help_detail.txt");

/// Manage Nodes
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    after_long_help = help::template(HELP_DETAIL)
)]
pub struct NodeCommand {
    #[command(subcommand)]
    subcommand: NodeSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum NodeSubcommand {
    #[command(display_order = 800)]
    Create(CreateCommand),
    #[command(display_order = 800)]
    Delete(DeleteCommand),
    #[command(display_order = 800)]
    List(ListCommand),
    #[command(display_order = 800)]
    Logs(LogCommand),
    Show(ShowCommand),
    #[command(display_order = 800)]
    Start(StartCommand),
    #[command(display_order = 800)]
    Stop(StopCommand),
    #[command(display_order = 800)]
    Default(DefaultCommand),
}

impl NodeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            NodeSubcommand::Create(c) => c.run(options),
            NodeSubcommand::Delete(c) => c.run(options),
            NodeSubcommand::List(c) => c.run(options),
            NodeSubcommand::Show(c) => c.run(options),
            NodeSubcommand::Start(c) => c.run(options),
            NodeSubcommand::Stop(c) => c.run(options),
            NodeSubcommand::Logs(c) => c.run(options),
            NodeSubcommand::Default(c) => c.run(options),
        }
    }
}

#[derive(Clone, Debug, Args)]
pub struct NodeOpts {
    /// Override the default API node
    #[arg(
        global = true,
        id = "node",
        value_name = "NODE",
        short,
        long,
        default_value_t = default_node_name()
    )]
    pub api_node: String,
}

pub fn default_node_name() -> String {
    CliState::try_default()
        .unwrap()
        .nodes
        .default()
        .map(|n| n.config.name)
        .unwrap_or_else(|_| "default".to_string())
}
