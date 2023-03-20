use crate::authority::create::CreateCommand;
use crate::{help, CommandGlobalOpts};
use clap::Args;
use clap::Subcommand;
mod create;

const HELP_DETAIL: &str = include_str!("../constants/authority/help_detail.txt");

/// Create an Authority node
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
subcommand_required = true,
after_long_help = help::template(HELP_DETAIL)
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
