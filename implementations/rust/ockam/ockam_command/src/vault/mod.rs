use clap::{Args, Subcommand};

pub use crate::vault::create::CreateCommand;
use crate::vault::delete::DeleteCommand;
use crate::vault::list::ListCommand;
use crate::vault::move_vault::MoveCommand;
use crate::vault::show::ShowCommand;
use crate::{docs, Command, CommandGlobalOpts};

use ockam_node::Context;

mod create;
mod delete;
mod list;
mod move_vault;
mod show;
mod util;

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
    pub subcommand: VaultSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum VaultSubcommand {
    Create(CreateCommand),
    Move(MoveCommand),
    Show(ShowCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl VaultCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            VaultSubcommand::Create(cmd) => cmd.run(ctx, opts).await,
            VaultSubcommand::Move(cmd) => cmd.run(opts).await,
            VaultSubcommand::Show(cmd) => cmd.run(opts).await,
            VaultSubcommand::List(cmd) => cmd.run(opts).await,
            VaultSubcommand::Delete(cmd) => cmd.run(opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            VaultSubcommand::Create(c) => c.name(),
            VaultSubcommand::Move(c) => c.name(),
            VaultSubcommand::Show(c) => c.name(),
            VaultSubcommand::Delete(c) => c.name(),
            VaultSubcommand::List(c) => c.name(),
        }
    }
}
