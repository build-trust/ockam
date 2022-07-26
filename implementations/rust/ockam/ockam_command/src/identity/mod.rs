mod create;
mod export;
mod print;

pub(crate) use create::CreateCommand;
pub(crate) use export::ExportCommand;
pub(crate) use print::PrintCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
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

    /// Print existing Identity Id
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Print(PrintCommand),

    /// Export existing Identity
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Export(ExportCommand),
}

impl IdentityCommand {
    pub fn run(opts: CommandGlobalOpts, command: IdentityCommand) {
        match command.subcommand {
            IdentitySubcommand::Create(command) => CreateCommand::run(opts, command),
            IdentitySubcommand::Print(command) => PrintCommand::run(opts, command),
            IdentitySubcommand::Export(command) => ExportCommand::run(opts, command),
        }
        .unwrap()
    }
}
