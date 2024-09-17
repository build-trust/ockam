use clap::{Args, Subcommand};

use create::InfluxDBCreateCommand;

use crate::{docs, Command, CommandGlobalOpts};

pub(crate) mod create;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage InfluxDB Inlets
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
)]
pub struct InfluxDBOutletCommand {
    #[command(subcommand)]
    pub subcommand: InfluxDBOutletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum InfluxDBOutletSubCommand {
    Create(InfluxDBCreateCommand),
}

impl InfluxDBOutletCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            InfluxDBOutletSubCommand::Create(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            InfluxDBOutletSubCommand::Create(c) => c.name(),
        }
    }
}
