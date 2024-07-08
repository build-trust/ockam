mod create;

use clap::{Args, Subcommand};

use create::CreateCommand;

use crate::{Command, CommandGlobalOpts};

/// Manage Rendezvous server
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct RendezvousCommand {
    #[command(subcommand)]
    pub subcommand: RendezvousSubcommand,
}

impl RendezvousCommand {
    pub fn name(&self) -> String {
        self.subcommand.name()
    }
}

#[derive(Clone, Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum RendezvousSubcommand {
    #[command(display_order = 800)]
    Create(CreateCommand),
}

impl RendezvousSubcommand {
    pub fn name(&self) -> String {
        match self {
            RendezvousSubcommand::Create(c) => c.name(),
        }
    }
}

impl RendezvousCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            RendezvousSubcommand::Create(c) => c.run(opts),
        }
    }
}
