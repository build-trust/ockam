mod create;
mod default;
mod delete;
mod list;
mod show;

use clap::{Args, Subcommand};

use crate::{docs, CommandGlobalOpts};

use crate::trust_context::default::DefaultCommand;
use crate::trust_context::delete::DeleteCommand;
use crate::trust_context::list::ListCommand;
use crate::trust_context::show::ShowCommand;
pub use create::CreateCommand;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage Trust Contexts
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
)]
pub struct TrustContextCommand {
    #[command(subcommand)]
    subcommand: TrustContextSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TrustContextSubcommand {
    Create(CreateCommand),
    Show(ShowCommand),
    Delete(DeleteCommand),
    List(ListCommand),
    Default(DefaultCommand),
}

impl TrustContextCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            TrustContextSubcommand::Create(c) => c.run(opts),
            TrustContextSubcommand::Show(cmd) => cmd.run(opts),
            TrustContextSubcommand::List(cmd) => cmd.run(opts),
            TrustContextSubcommand::Delete(cmd) => cmd.run(opts),
            TrustContextSubcommand::Default(cmd) => cmd.run(opts),
        }
    }
}
