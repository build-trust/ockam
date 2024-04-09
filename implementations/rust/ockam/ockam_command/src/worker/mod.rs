use clap::{Args, Subcommand};

use list::ListCommand;

use crate::{docs, CommandGlobalOpts};

mod list;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage Workers
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            WorkerSubcommand::List(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            WorkerSubcommand::List(c) => c.name(),
        }
    }
}
