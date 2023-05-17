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
    long_about = docs::after_help(LONG_ABOUT),
)]
pub struct AuthorityCommand {
    #[command(subcommand)]
    subcommand: AuthoritySubcommand,
}

impl AuthorityCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            AuthoritySubcommand::Create(c) => c.run(options),
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum AuthoritySubcommand {
    #[command(display_order = 800)]
    Create(CreateCommand),
}
