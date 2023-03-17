use crate::{docs, CommandGlobalOpts};
use clap::{Args, Subcommand};

use list::ListCommand;

mod list;

const HELP_DETAIL: &str = "";

/// Manage Workers
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    after_long_help = docs::after_help(HELP_DETAIL)
)]
pub struct WorkerCommand {
    #[command(subcommand)]
    subcommand: WorkerSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum WorkerSubcommand {
    #[command(display_order = 800)]
    List(ListCommand),
}

impl WorkerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            WorkerSubcommand::List(c) => c.run(options),
        }
    }
}
