use clap::{Args, Subcommand};

pub(crate) use create::CreateCommand;

use crate::{help, CommandGlobalOpts};

mod create;

const HELP_DETAIL: &str = "\
EXAMPLES:

```sh
    # Create two nodes
    $ ockam node create n1
    $ ockam node create n2

    # Create a forwarder to node n2 at node n1
    $ ockam forwarder create --from forwarder_to_n2 --for /node/n2 --at /node/n1

    # Send message via the forwarder
    $ ockam message send hello --to /node/n1/service/forwarder_to_n2/service/uppercase
```
";

/// Manage Forwarders
#[derive(Clone, Debug, Args)]
#[clap(help_template = help::template(HELP_DETAIL))]
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
