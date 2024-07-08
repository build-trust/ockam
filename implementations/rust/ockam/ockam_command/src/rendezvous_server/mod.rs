mod start;

use clap::{Args, Subcommand};

use start::StartCommand;

use crate::{Command, CommandGlobalOpts};

/// Manage Rendezvous server
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct RendezvousServerCommand {
    #[command(subcommand)]
    pub subcommand: RendezvousServerSubcommand,
}

impl RendezvousServerCommand {
    pub fn name(&self) -> String {
        self.subcommand.name()
    }
}

#[derive(Clone, Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum RendezvousServerSubcommand {
    #[command(display_order = 800)]
    Start(StartCommand),
}

impl RendezvousServerSubcommand {
    pub fn name(&self) -> String {
        match self {
            RendezvousServerSubcommand::Start(c) => c.name(),
        }
    }
}

impl RendezvousServerCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            RendezvousServerSubcommand::Start(c) => c.run(opts),
        }
    }
}
