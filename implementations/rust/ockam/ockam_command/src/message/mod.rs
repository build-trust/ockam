use crate::{help, CommandGlobalOpts};
use clap::{Args, Subcommand};
pub use send::SendCommand;

mod send;

const HELP_DETAIL: &str = "\
About:
    An Ockam node is any running application that can communicate with other applications
    using various Ockam protocols like Routing, Secure Channels, Forwarding etc.

    We can create Ockam nodes using this command line or using various Ockam programming
    libraries like our Rust and Elixir libraries.

    Workers
    ------

    Ockam nodes run very lightweight, concurrent, stateful actors called Ockam Workers.
    Workers have addresses and a node can deliver messages to workers on the same node or
    on a different node using the Ockam Routing Protocol and its Transports.


    Routing
    ------

    The Ockam Routing Protocol is a very simple application layer protocol that allows
    the sender of a message to describe the `onward_route` and `return_route` of message.

    The routing layer in a node can then be used route these messages between workers within
    a node or across nodes using transports. Messages can be sent over multiple hops, within
    one node or across many nodes.


    Transports
    ------

    Transports are plugins to the Ockam Routing layer that allow Ockam Routing messages
    to travel across nodes over transport layer protocols like TCP, UDP, BLUETOOTH etc.


    Services
    ------

    One or more Ockam Workers can work as a team to offer a Service. Services have
    addressed represented by /service/{ADDRESS}. Services can be attached to identities and
    authorization policies to enforce attribute based access control rules.

    Nodes created using `ockam` command usually start a pre-defined set of default services.

    This includes:
        - A uppercase service at /service/uppercase
        - A secure channel listener at /service/api
        - A tcp listener listening at some TCP port

Examples:

```sh
    # Create two nodes
    $ ockam node create n1
    $ ockam node create n2

    # Send a message to the uppercase service on node 2
    $ ockam message send hello --to /node/n2/service/uppercase
    HELLO

    # A more verbose version of the above would be,
    # assuming n2 started its tcp listener on port 4000.
    $ ockam message send hello --to /ip4/127.0.0.1/tcp/4000/service/uppercase
    HELLO

    # Send a message to the uppercase service on node n2 from node n1
    $ ockam message send hello --from /node/n1 --to /node/n2/service/uppercase
    HELLO

    # Create a secure channel from node n1 to the api service on node n2
    # The /service/api is a secure channel listener that is started on every node
    # Send a message through this encrypted channel to the uppercase service
    $ ockam secure-channel create --from /node/n1 --to /node/n2/service/api \\
        | ockam message send hello --from /node/n1 --to -/service/uppercase
    HELLO
```
";

/// Send and Receive Messages
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    help_template = help::template(HELP_DETAIL)
)]
pub struct MessageCommand {
    #[command(subcommand)]
    subcommand: MessageSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum MessageSubcommand {
    #[command(display_order = 800)]
    Send(SendCommand),
}

impl MessageCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            MessageSubcommand::Send(c) => c.run(options),
        }
    }
}
