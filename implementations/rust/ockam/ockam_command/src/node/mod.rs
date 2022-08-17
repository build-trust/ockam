mod create;
mod delete;
mod list;
mod show;
mod start;
mod stop;

pub(crate) use create::CreateCommand;
use delete::DeleteCommand;
use list::ListCommand;
use show::ShowCommand;
use start::StartCommand;
use stop::StopCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

const EXAMPLES: &str = "\
EXAMPLES

    # Create a node
    $ ockam node create n1

    # Send a message to the uppercase service worker in that node
    $ ockam message send \"hello ockam\" --to /node/n1/service/uppercase
    HELLO OCKAM

    # Delete the node
    $ ockam node delete n1

LEARN MORE
";

#[derive(Clone, Debug, Args)]
/// Manage nodes
///
/// An Ockam node is any running application that can communicate with other
/// applications using various Ockam protocols like Routing, Secure Channels, Forwarding etc.
///
/// Ockam nodes run very lightweight, concurrent, stateful actors called Ockam Workers.
/// Workers have addresses and a node can deliver messages to workers on the same node or on
/// a different node.
#[clap(help_template = const_str::replace!(HELP_TEMPLATE, "LEARN MORE", EXAMPLES))]
pub struct NodeCommand {
    #[clap(subcommand)]
    subcommand: NodeSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum NodeSubcommand {
    #[clap(display_order = 900)]
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

    /// Start a, previously created, node.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Start(StartCommand),

    /// Stop a, previously created, node.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Stop(StopCommand),
}

impl NodeCommand {
    pub fn run(opts: CommandGlobalOpts, command: NodeCommand) {
        match command.subcommand {
            NodeSubcommand::Create(command) => CreateCommand::run(opts, command),
            NodeSubcommand::Delete(command) => DeleteCommand::run(opts, command),
            NodeSubcommand::List(command) => ListCommand::run(opts, command),
            NodeSubcommand::Show(command) => ShowCommand::run(opts, command),
            NodeSubcommand::Start(command) => StartCommand::run(opts, command),
            NodeSubcommand::Stop(command) => StopCommand::run(opts, command),
        }
    }
}

#[derive(Clone, Debug, Args)]
pub struct NodeOpts {
    /// Override the default API node
    #[clap(global = true, name = "node", short, long, default_value = "default")]
    pub api_node: String,
}

pub fn random_name() -> String {
    hex::encode(&rand::random::<[u8; 4]>())
}
