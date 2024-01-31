use clap::{command, Args, Subcommand};

use crate::kafka::direct::create::CreateCommand;
use crate::kafka::direct::delete::DeleteCommand;
use crate::kafka::direct::list::ListCommand;
use crate::CommandGlobalOpts;

pub(crate) mod command;
mod create;
mod delete;
mod list;

/// Manage Kafka Consumers
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct KafkaDirectCommand {
    #[command(subcommand)]
    subcommand: KafkaDirectSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum KafkaDirectSubcommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl KafkaDirectCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            KafkaDirectSubcommand::Create(c) => c.run(opts),
            KafkaDirectSubcommand::Delete(c) => c.run(opts),
            KafkaDirectSubcommand::List(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            KafkaDirectSubcommand::Create(c) => c.name(),
            KafkaDirectSubcommand::Delete(c) => c.name(),
            KafkaDirectSubcommand::List(c) => c.name(),
        }
        .to_string()
    }
}
