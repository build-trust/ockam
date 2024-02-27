use clap::{Args, Subcommand};

pub use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use crate::identity::default::DefaultCommand;
use crate::{docs, Command, CommandGlobalOpts};

mod create;
mod default;
mod delete;
mod list;
mod show;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage Identities
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
subcommand_required = true,
long_about = docs::about(LONG_ABOUT),
)]
pub struct IdentityCommand {
    #[command(subcommand)]
    pub subcommand: IdentitySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum IdentitySubcommand {
    Create(CreateCommand),
    Show(ShowCommand),
    List(ListCommand),
    Default(DefaultCommand),
    Delete(DeleteCommand),
}

impl IdentityCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            IdentitySubcommand::Create(c) => c.run(opts),
            IdentitySubcommand::Show(c) => c.run(opts),
            IdentitySubcommand::List(c) => c.run(opts),
            IdentitySubcommand::Delete(c) => c.run(opts),
            IdentitySubcommand::Default(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            IdentitySubcommand::Create(c) => c.name(),
            IdentitySubcommand::Show(c) => c.name(),
            IdentitySubcommand::List(c) => c.name(),
            IdentitySubcommand::Delete(c) => c.name(),
            IdentitySubcommand::Default(c) => c.name(),
        }
        .to_string()
    }
}
