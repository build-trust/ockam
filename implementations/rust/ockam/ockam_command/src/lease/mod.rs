mod create;
mod influxdb;
mod list;
mod show;
pub use create::CreateCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use clap::{Args, Subcommand};

use crate::{util::api::CloudOpts, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct LeaseCommand {
    #[command(subcommand)]
    subcommand: LeaseSubcommand,

    #[command(flatten)]
    lease_args: LeaseArgs,
}

#[derive(Clone, Debug, Args)]
pub struct LeaseArgs {
    #[arg(long, short)]
    project_name: String,

    #[command(flatten)]
    cloud_opts: CloudOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum LeaseSubcommand {
    Create(CreateCommand),
    List(ListCommand),
    Show(ShowCommand),
}

impl LeaseCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            LeaseSubcommand::Create(c) => c.run(options, self.lease_args),
            LeaseSubcommand::List(c) => c.run(options, self.lease_args),
            LeaseSubcommand::Show(c) => c.run(options, self.lease_args),
        }
    }
}
