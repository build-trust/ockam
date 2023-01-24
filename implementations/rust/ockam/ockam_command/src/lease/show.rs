use clap::{Args, Subcommand};

use crate::{help, CommandGlobalOpts};

use crate::lease::influxdb::InfluxDbShowCommand;
use super::LeaseArgs;

const HELP_DETAIL: &str = "";

/// Show detailed token information within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = help::template(HELP_DETAIL))]
pub struct ShowCommand {
    #[command(subcommand)]
    subcommand: ShowSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ShowSubcommand {
    InfluxDb(InfluxDbShowCommand),
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts, lease_args: LeaseArgs) {
        match self.subcommand {
            ShowSubcommand::InfluxDb(c) => c.run(options, lease_args),
        }
    }
}
