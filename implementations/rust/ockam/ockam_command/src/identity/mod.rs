mod create;

pub(crate) use create::CreateCommand;

use crate::{util::OckamConfig, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct IdentityCommand {
    #[clap(subcommand)]
    subcommand: IdentitySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum IdentitySubcommand {
    /// Create Identity
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),
}

impl IdentityCommand {
    pub fn run(cfg: &OckamConfig, command: IdentityCommand) {
        match command.subcommand {
            IdentitySubcommand::Create(command) => CreateCommand::run(cfg, command),
        }
        .unwrap()
    }
}
