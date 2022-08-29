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
EXAMPLES

```sh
    # Create a node
    $ ockam node create n1

    # Send a message to the uppercase service worker in that node
    $ ockam message send \"hello ockam\" --to /node/n1/service/uppercase
    HELLO OCKAM

    # Delete the node
    $ ockam node delete n1
```
";

/// Manage Nodes
///
/// An Ockam node is any running application that can communicate with other
/// applications using various Ockam protocols like Routing, Secure Channels, Forwarding etc.
///
/// Ockam nodes run very lightweight, concurrent, stateful actors called Ockam Workers.
/// Workers have addresses and a node can deliver messages to workers on the same node or on
/// a different node.
#[derive(Clone, Debug, Args)]
#[clap(help_template = help::template(HELP_DETAIL))]
pub struct NodeCommand {
    #[clap(subcommand)]
    subcommand: NodeSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum NodeSubcommand {
    Create(CreateCommand),
    Delete(DeleteCommand),

    List(ListCommand),

    Show(ShowCommand),

    Start(StartCommand),

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
