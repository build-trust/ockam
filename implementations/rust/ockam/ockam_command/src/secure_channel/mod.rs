pub(crate) mod listener;

mod create;
mod delete;
mod list;
mod show;

pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use crate::{help, CommandGlobalOpts};
use clap::{Args, Subcommand};

const HELP_DETAIL: &str = "\
ABOUT:
    Secure Channels provide end-to-end encrypted and mutually authenticated communication
    that is safe against eavesdropping, tampering, and forgery of messages en-route.

    To create a secure channel, we first need a secure channel listener. Every node that
    is started with ockam command, by convention, starts a secure channel listener at the
    address /service/api.

    So the simplest example of creating a secure channel would be:

```sh
    $ ockam node create n1
    $ ockam node create n1

    $ ockam secure-channel create --from /node/n1 --to /node/n2/service/api
    /service/09738b73c54b81d48531f659aaa22533
```

    The Ockam Secure Channels protocol is based on handshake designs proposed in the
    Noise Protocol Framework. The Noise framework proposes several handshake designs
    that make different tradeoffs to achieve various security properties like mutual
    authentication, forward secrecy, and resistance to key compromise impersonation etc.
    These design have been scrutinized by many experts and have, openly published,
    formal proofs.

    Ockam Secure Channels protocol is an opinionated implementation of one such proven
    design and `ockam` command makes it super simple to create mutually authenticated
    noise based secure channels.

    This secure channels protocol is layered above Ockam Routing and is decoupled
    from transport protocols like TCP, UDP, Bluetooth etc. This allows Ockam Secure Channels
    to be end-to-end over multiple transport layer hops.

    For instance we can create a secure channel over two TCP connection hops, as follows,
    and then send a message through it.

```sh
    # Create three nodes and make them start tcp transport listeners at specific ports
    $ ockam node create n1 --tcp-listener-address 127.0.0.1:6001
    $ ockam node create n2 --tcp-listener-address 127.0.0.1:6002
    $ ockam node create n3 --tcp-listener-address 127.0.0.1:6003

    $ ockam secure-channel create --from /node/n1 \\
        --to /ip4/127.0.0.1/tcp/6002/ip4/127.0.0.1/tcp/6003/service/api \\
          | ockam message send hello --from /node/n1 --to -/service/uppercase
    HELLO

    # Or the more concise:
    $ ockam secure-channel create --from /node/n1 --to /node/n2/node/n3/service/api \\
        | ockam message send hello --from /node/n1 --to -/service/uppercase
    HELLO
```


    Combining Secure Channels and Forwarders
    ------

    We can also create a secure channel through Ockam Forwarders.

    Forwarders enable an ockam node to register a forwarding address on another node.
    Any message that arrives at this forwarding address is immediately dispatched
    to the node that registered the forwarding address.

```sh
    # Create three nodes
    $ ockam node create relay

    # Create a forwarder to node n2 at node relay
    $ ockam forwarder create blue --at /node/relay --to /node/n2
    /service/forward_to_n2

    # Create an end-to-end secure channel between n1 and n2.
    # This secure channel is created trough n2's forwarder at relay and we can
    # send end-to-end encrypted messages through it.
    $ ockam secure-channel create --from /node/n1 --to /node/relay/service/forward_to_n2/service/api \\
        | ockam message send hello --from /node/n1 --to -/service/uppercase
```

    In this topology `relay` acts an an encrypted relay between n1 and n2. n1 and
    n2 can be running in completely separate private networks. The relay only sees encrypted
    traffic and needs to be reachable from both n1 and n2.

    This can be very useful in establishing end-to-end trustful communication between
    applications that cannot otherwise reach each other over the network.

    For instance, we can use forwarders to create an end-to-end secure channel between
    two nodes that are behind private NATs.


    List Secure Channels initiated from a node
    ------

```sh
    $ ockam secure-channel list --node n1
```


    Delete Secure Channels initiated from a node
    ------

```sh
    $ ockam secure-channel delete 5f84acc6bf4cb7686e3103555980c05b --at n1
```


    Custom Secure Channel Listeners
    ------

    All node start with a secure channel listener at `/service/api` but you can also
    start a custom listener with specific authorization policies.

```sh
    # Create a secure channel listener on n1
    $ ockam secure-channel-listener create test --at n2
    /service/test

    # Create a secure channel listener from n1 to our test secure channel listener on n2
    $ ockam secure-channel create --from /node/n1 --to /node/n2/service/test
    /service/09738b73c54b81d48531f659aaa22533
```
";

/// Manage Secure Channels.
#[derive(Clone, Debug, Args)]
#[clap(
    arg_required_else_help = true,
    subcommand_required = true,
    help_template = help::template(HELP_DETAIL),
    mut_subcommand("help", |c| c.about("Print help information"))
)]
pub struct SecureChannelCommand {
    #[clap(subcommand)]
    subcommand: SecureChannelSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
enum SecureChannelSubcommand {
    #[clap(display_order = 800)]
    Create(CreateCommand),
    #[clap(display_order = 800)]
    Delete(DeleteCommand),
    #[clap(display_order = 800)]
    List(ListCommand),
    #[clap(display_order = 800)]
    Show(ShowCommand),
}

impl SecureChannelCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            SecureChannelSubcommand::Create(c) => c.run(options),
            SecureChannelSubcommand::Delete(c) => c.run(options),
            SecureChannelSubcommand::List(c) => c.run(options),
            SecureChannelSubcommand::Show(c) => c.run(options),
        }
    }
}
