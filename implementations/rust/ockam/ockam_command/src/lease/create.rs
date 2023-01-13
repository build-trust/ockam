use clap::{Args, Subcommand};

use crate::{help, CommandGlobalOpts};

use super::{influxdb::InfluxDbCreateCommand, LeaseArgs};

/// Create a token within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct CreateCommand {
    #[command(subcommand)]
    subcommand: CreateSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CreateSubcommand {
    InfluxDb(InfluxDbCreateCommand),
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts, lease_args: LeaseArgs) {
        match self.subcommand {
            CreateSubcommand::InfluxDb(c) => c.run(options, lease_args),
        }
    }
}
