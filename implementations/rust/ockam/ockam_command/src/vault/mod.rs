mod create;

pub(crate) use create::CreateCommand;

use crate::help;
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct VaultCommand {
    #[command(subcommand)]
    subcommand: VaultSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum VaultSubcommand {
    Create(CreateCommand),
}

impl VaultCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            VaultSubcommand::Create(c) => c.run(options),
        }
        .unwrap()
    }
}
