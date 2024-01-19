mod get;
mod list;

use get::GetCommand;
use list::ListCommand;

use crate::docs;
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide(), after_long_help = docs::after_help(HELP_DETAIL))]
pub struct ConfigurationCommand {
    #[command(subcommand)]
    subcommand: ConfigurationSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ConfigurationSubcommand {
    Get(GetCommand),
    List(ListCommand),
}

impl ConfigurationCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            ConfigurationSubcommand::Get(c) => c.run(opts),
            ConfigurationSubcommand::List(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            ConfigurationSubcommand::Get(c) => c.name(),
            ConfigurationSubcommand::List(c) => c.name(),
        }
    }
}
