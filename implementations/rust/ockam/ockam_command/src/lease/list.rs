use clap::{Args, Subcommand};

use crate::{help, CommandGlobalOpts};

use super::{influxdb::InfluxDbListCommand, LeaseArgs};

const HELP_DETAIL: &str = "";

/// List tokens within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = help::template(HELP_DETAIL))]
pub struct ListCommand {
    #[command(subcommand)]
    subcommand: ListSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ListSubcommand {
    InfluxDb(InfluxDbListCommand),
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts, lease_args: LeaseArgs) {
        match self.subcommand {
            ListSubcommand::InfluxDb(c) => c.run(options, lease_args),
        }
    }
}
