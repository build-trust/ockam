use crate::authority::create::CreateCommand;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use clap::Subcommand;
mod create;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage Authority nodes
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
)]
pub struct AuthorityCommand {
    #[command(subcommand)]
    pub(crate) subcommand: AuthoritySubcommand,
}

impl AuthorityCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            AuthoritySubcommand::Create(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            AuthoritySubcommand::Create(c) => c.name(),
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum AuthoritySubcommand {
    #[command(display_order = 800)]
    Create(CreateCommand),
}
