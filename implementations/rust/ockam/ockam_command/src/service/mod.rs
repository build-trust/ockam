pub(crate) mod config;
pub(crate) mod start;

pub(crate) use start::StartCommand;

use crate::help;
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
#[clap(hide = help::hide())]
pub struct ServiceCommand {
    #[clap(subcommand)]
    subcommand: ServiceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ServiceSubcommand {
    #[clap(display_order = 900)]
    Start(StartCommand),
}

impl ServiceCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            ServiceSubcommand::Start(c) => c.run(options),
        }
        .unwrap()
    }
}
