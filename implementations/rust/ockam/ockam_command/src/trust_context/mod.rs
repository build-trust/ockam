mod create;
use clap::{Args, Subcommand};

pub use create::CreateCommand;

use crate::{util::api::TrustContextOpts, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
/// An Ockam Trust Context defines which authorities are trusted to attest to which attributes.
///
/// Trust Contexts can be defined when creating a new node and resources, by supplying the given name or path.
/// A default trust context can be created and will be used when an explicit trust context is not provided.
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
