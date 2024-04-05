use clap::{command, Args, Subcommand};

use crate::kafka::inlet::create::CreateCommand;
use crate::{Command, CommandGlobalOpts};

pub(crate) mod create;

/// Manage Kafka Inlets
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct KafkaInletCommand {
    #[command(subcommand)]
    pub(crate) subcommand: KafkaInletSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum KafkaInletSubcommand {
    Create(CreateCommand),
}

impl KafkaInletCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            KafkaInletSubcommand::Create(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            KafkaInletSubcommand::Create(c) => c.name(),
        }
    }
}
