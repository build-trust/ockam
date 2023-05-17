mod create;
use clap::{Args, Subcommand};

pub use create::CreateCommand;

use crate::{docs, util::api::TrustContextOpts, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage trust contexts
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
)]
pub struct TrustContextCommand {
    #[command(subcommand)]
    subcommand: TrustContextSubcommand,

    #[command(flatten)]
    trust_context_opts: TrustContextOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TrustContextSubcommand {
    Create(CreateCommand),
}

impl TrustContextCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TrustContextSubcommand::Create(c) => c.run(options, self.trust_context_opts),
        }
    }
}
