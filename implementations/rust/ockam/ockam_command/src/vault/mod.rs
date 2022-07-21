mod create;

pub(crate) use create::CreateCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct VaultCommand {
    #[clap(subcommand)]
    subcommand: VaultSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum VaultSubcommand {
    /// Create Vault
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),
}

impl VaultCommand {
    pub fn run(opts: CommandGlobalOpts, command: VaultCommand) {
        match command.subcommand {
            VaultSubcommand::Create(command) => CreateCommand::run(opts, command),
        }
        .unwrap()
    }
}
