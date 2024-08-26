use clap::{Args, Subcommand};

use create::CreateCommand;

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
pub struct InfluxDBInletCommand {
    #[command(subcommand)]
    pub subcommand: InfluxDBInletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum InfluxDBInletSubCommand {
    Create(CreateCommand),
}

impl InfluxDBInletCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            InfluxDBInletSubCommand::Create(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            InfluxDBInletSubCommand::Create(c) => c.name(),
        }
    }
}
