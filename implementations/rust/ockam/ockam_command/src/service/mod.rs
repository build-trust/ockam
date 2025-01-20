pub(crate) mod config;

pub(crate) mod list;
pub(crate) mod start;

use crate::{docs, CommandGlobalOpts};
use clap::{Args, Subcommand};
use list::ListCommand;
pub(crate) use start::StartCommand;

#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide())]
pub struct ServiceCommand {
    #[command(subcommand)]
    subcommand: ServiceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ServiceSubcommand {
    #[command(display_order = 900)]
    Start(StartCommand),
    #[command(display_order = 901)]
    List(ListCommand),
}

impl ServiceCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            ServiceSubcommand::Start(c) => c.run(opts),
            ServiceSubcommand::List(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            ServiceSubcommand::Start(c) => c.name(),
            ServiceSubcommand::List(c) => c.name(),
        }
    }
}
