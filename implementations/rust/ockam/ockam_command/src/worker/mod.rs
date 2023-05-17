use crate::{docs, CommandGlobalOpts};
use clap::{Args, Subcommand};

use list::ListCommand;

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
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            WorkerSubcommand::List(c) => c.run(options),
        }
    }
}
