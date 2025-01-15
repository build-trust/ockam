use clap::{Args, Subcommand};

use crate::{docs, Command, CommandGlobalOpts};

use create::CreateCommand;

use ockam_node::Context;

pub(crate) mod create;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage InfluxDB Inlets
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
)]
pub struct InfluxDBInletCommand {
    #[command(subcommand)]
    pub subcommand: InfluxDBInletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum InfluxDBInletSubCommand {
    Create(CreateCommand),
}

impl InfluxDBInletCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            InfluxDBInletSubCommand::Create(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            InfluxDBInletSubCommand::Create(c) => c.name(),
        }
    }
}
