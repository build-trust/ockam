pub(crate) mod listener;

mod create;
mod delete;
mod list;

pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use list::ListCommand;

use crate::{help, CommandGlobalOpts};
use clap::{Args, Subcommand};

/// Manage Secure Channels.
///
/// Secure Channels provide end-to-end encrypted and mutually authenticated
/// communication that is safe against eavesdropping, tampering, and forgery
/// of messages en-route.
#[derive(Clone, Debug, Args)]
#[clap(
    arg_required_else_help = true,
    subcommand_required = true,
    help_template = help::template(HELP_DETAIL)
)]
pub struct SecureChannelCommand {
    #[clap(subcommand)]
    subcommand: SecureChannelSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
enum SecureChannelSubcommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl SecureChannelCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            SecureChannelSubcommand::Create(c) => c.run(options),
            SecureChannelSubcommand::Delete(c) => c.run(options),
            SecureChannelSubcommand::List(c) => c.run(options),
        }
    }
}

const HELP_DETAIL: &str = "\
BACKGROUND:

Secure Channels provide end-to-end encrypted and mutually authenticated
communication that is safe against eavesdropping, tampering, and forgery
of messages en-route.

To create a secure channel, we first need a secure channel listener.
Every node that is started with ockam command, by default, starts a secure
channel listener at the address /service/api.

So the simplest example of creating a secure channel would be:

```sh
    $ ockam node create n1
    $ ockam node create n2
    $ ockam secure-channel create --from /node/n1 --to /node/n2/service/api
    /service/09738b73c54b81d48531f659aaa22533
```

The Ockam Secure Channels protocol is based on handshake designs proposed
in the Noise Protocol Framework.

Ockam Secure Channels protocol is layered above Ockam Routing and is
decoupled from transport protocols like TCP, UDP, Bluetooth etc. This allows
Ockam Secure Channels to be end-to-end over multiple transport layer hops.

For instance we can create a secure channel over two TCP connection hops
as follows:

```sh
    $ ockam node create n1
    $ ockam node create n2
    $ ockam node create n3

    $ ockam secure-channel create --from /node/n1 --to /node/n2/node/n3/service/api \\
        | ockam message send hello --from /node/n1 --to -/service/uppercase
    HELLO
```
";
