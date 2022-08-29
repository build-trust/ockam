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

use crate::{help, CommandGlobalOpts};
use clap::{Args, Subcommand};

const HELP_DETAIL: &str = "\
ABOUT:

An Ockam node is any running application that can communicate with other
applications using various Ockam protocols like Routing, Secure Channels, Forwarding etc.

Ockam nodes run very lightweight, concurrent, stateful actors called Ockam Workers.
Workers have addresses and a node can deliver messages to workers on the same node or on
a different node.

EXAMPLES:
```sh
    # Create a node
    $ ockam node create n1

    # Send a message to the uppercase service worker in that node
    $ ockam message send \"hello ockam\" --to /node/n1/service/uppercase
    HELLO OCKAM

    # Create a node, with a specified tcp listener address
    $ ockam node create n1 --tcp-listener-address 127.0.0.1:6001

    # Create a node, and run it in the foreground with verbose traces
    $ ockam node create n1 --foreground -vvv

    # Show information about a specific node
    $ ockam node show n1

    # List all created nodes
    $ ockam node list

    # Delete the node
    $ ockam node delete n1

    # Delete all nodes
    $ ockam node delete --all

    # Delete all nodes and force cleanup
    $ ockam node delete --all --force
```
";

/// Manage Nodes
#[derive(Clone, Debug, Args)]
#[clap(
    arg_required_else_help = true,
    subcommand_required = true,
    help_template = help::template(HELP_DETAIL),
    mut_subcommand("help", |c| c.about("Print help information"))
)]
pub struct NodeCommand {
    #[clap(subcommand)]
    subcommand: NodeSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum NodeSubcommand {
    #[clap(display_order = 800)]
    Create(CreateCommand),
    #[clap(display_order = 800)]
    Delete(DeleteCommand),
    #[clap(display_order = 800)]
    List(ListCommand),
    #[clap(display_order = 800)]
    Show(ShowCommand),
    #[clap(display_order = 800)]
    Start(StartCommand),
    #[clap(display_order = 800)]
    Stop(StopCommand),
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
