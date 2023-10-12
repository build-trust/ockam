mod create;
mod default;
mod delete;
mod list;
mod show;

use crate::vault::create::CreateCommand;
use crate::vault::default::DefaultCommand;
use crate::vault::delete::DeleteCommand;
use crate::vault::list::ListCommand;
use crate::vault::show::ShowCommand;
use crate::{docs, CommandGlobalOpts};

use clap::{Args, Subcommand};
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::CliState;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage Vaults
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
)]
pub struct VaultCommand {
    #[command(subcommand)]
    subcommand: VaultSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum VaultSubcommand {
    Create(CreateCommand),
    Show(ShowCommand),
    Delete(DeleteCommand),
    List(ListCommand),
    Default(DefaultCommand),
}

impl VaultCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            VaultSubcommand::Create(cmd) => cmd.run(opts),
            VaultSubcommand::Show(cmd) => cmd.run(opts),
            VaultSubcommand::List(cmd) => cmd.run(opts),
            VaultSubcommand::Delete(cmd) => cmd.run(opts),
            VaultSubcommand::Default(cmd) => cmd.run(opts),
        }
    }
}

pub fn default_vault_name(cli_state: &CliState) -> String {
    cli_state
        .vaults
        .default()
        .map_or("default".to_string(), |v| v.name().to_string())
}
