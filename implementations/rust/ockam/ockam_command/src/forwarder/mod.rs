use clap::{Args, Subcommand};

pub(crate) use create::CreateCommand;

use crate::{help, CommandGlobalOpts};

mod create;

const HELP_DETAIL: &str = "\
ABOUT:
    Forwarders enable an ockam node to register a forwarding address on another node.
    Any message that arrives at this forwarding address is immediately dispatched
    to the node that registered the forwarding address.

```sh
    # Create two nodes blue and green
    $ ockam node create blue
    $ ockam node create green

    # Create a forwarder to node n2 at node n1
    $ ockam forwarder create blue --at /node/green --to /node/blue
    /service/forward_to_blue

    # Send a message to the uppercase service on blue via its forwarder on green
    $ ockam message send hello --to /node/green/service/forward_to_blue/service/uppercase
```

    This can be very useful in establishing communication between applications
    that cannot otherwise reach each other over the network.

    For instance, we can use forwarders to create an end-to-end secure channel between
    two nodes that are behind private NATs

```sh
    # Create another node called yellow
    $ ockam node create yellow

    # Create an end-to-end secure channel between yellow and blue.
    # This secure channel is created trough blue's forwarder at green and we can
    # send end-to-end encrypted messages through it.
    $ ockam secure-channel create --from /node/yellow --to /node/green/service/forward_to_blue/service/api \\
        | ockam message send hello --from /node/yellow --to -/service/uppercase
```

    In this topology green acts an an encrypted relay between yellow and blue. Yellow and
    blue can be running in completely separate private networks. Green needs to be reachable
    from both yellow and blue and only sees encrypted traffic.
";

/// Manage Forwarders
#[derive(Clone, Debug, Args)]
#[clap(
    arg_required_else_help = true,
    subcommand_required = true,
    help_template = help::template(HELP_DETAIL)
)]
pub struct ForwarderCommand {
    #[clap(subcommand)]
    subcommand: ForwarderSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ForwarderSubCommand {
    Create(CreateCommand),
}

impl ForwarderCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            ForwarderSubCommand::Create(c) => c.run(opts),
        }
    }
}
