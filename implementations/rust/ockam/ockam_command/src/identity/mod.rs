mod create;
mod show;

pub(crate) use create::CreateCommand;
pub(crate) use show::ShowCommand;

use crate::{hide, CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct IdentityCommand {
    #[clap(subcommand)]
    subcommand: IdentitySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum IdentitySubcommand {
    /// Create Identity
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, hide = hide())]
    Create(CreateCommand),
    /// Print short existing identity, `--full` for long identity
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Show(ShowCommand),
}

impl IdentityCommand {
    pub fn run(opts: CommandGlobalOpts, command: IdentityCommand) {
        match command.subcommand {
            IdentitySubcommand::Create(command) => CreateCommand::run(opts, command),
            IdentitySubcommand::Show(command) => ShowCommand::run(opts, command),
        }
        .unwrap()
    }
}
