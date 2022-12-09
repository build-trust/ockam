use clap::{Args, Subcommand};

pub(crate) use create::CreateCommand;

use crate::{help, CommandGlobalOpts};

mod create;

const HELP_DETAIL: &str = include_str!("../constants/forwarder/help_detail.txt");

/// Manage Forwarders
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    after_long_help = help::template(HELP_DETAIL)
)]
pub struct ForwarderCommand {
    #[command(subcommand)]
    subcommand: ForwarderSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ForwarderSubCommand {
    Create(CreateCommand),
}

impl ForwarderCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            ForwarderSubCommand::Create(c) => c.run(opts),
        }
    }
}
