use clap::{Args, Subcommand};

use crate::CommandGlobalOpts;

use super::{influxdb::InfluxDbRevokeCommand, LeaseArgs};

/// Revoke a token within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct RevokeCommand {
    #[command(subcommand)]
    subcommand: RevokeSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum RevokeSubcommand {
    InfluxDb(InfluxDbRevokeCommand),
}

impl RevokeCommand {
    pub fn run(self, options: CommandGlobalOpts, lease_args: LeaseArgs) {
        match self.subcommand {
            RevokeSubcommand::InfluxDb(c) => c.run(options, lease_args),
        }
    }
}
