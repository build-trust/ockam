mod get;
mod get_default_node;
mod list;
mod set_default_node;

use get::GetCommand;
use get_default_node::GetDefaultNodeCommand;
use list::ListCommand;
use set_default_node::SetDefaultNodeCommand;

use crate::help;
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide(), after_long_help = help::template(HELP_DETAIL))]
pub struct ConfigurationCommand {
    #[command(subcommand)]
    subcommand: ConfigurationSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ConfigurationSubcommand {
    Get(GetCommand),
    GetDefaultNode(GetDefaultNodeCommand),
    List(ListCommand),
    SetDefaultNode(SetDefaultNodeCommand),
}

impl ConfigurationCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            ConfigurationSubcommand::Get(c) => c.run(options),
            ConfigurationSubcommand::GetDefaultNode(c) => c.run(options),
            ConfigurationSubcommand::List(c) => c.run(options),
            ConfigurationSubcommand::SetDefaultNode(c) => c.run(options),
        }
    }
}
