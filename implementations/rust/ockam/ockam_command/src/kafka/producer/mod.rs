use clap::{command, Args, Subcommand};

use crate::kafka::producer::create::CreateCommand;
use crate::kafka::producer::delete::DeleteCommand;
use crate::kafka::producer::list::ListCommand;
use crate::CommandGlobalOpts;

mod create;
mod delete;
mod list;

/// Manage Kafka Producers [DEPRECATED]
#[derive(Clone, Debug, Args)]
#[command(hide = true, arg_required_else_help = true, subcommand_required = true)]
pub struct KafkaProducerCommand {
    #[command(subcommand)]
    subcommand: KafkaProducerSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum KafkaProducerSubcommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl KafkaProducerCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            KafkaProducerSubcommand::Create(c) => c.run(opts),
            KafkaProducerSubcommand::Delete(c) => c.run(opts),
            KafkaProducerSubcommand::List(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            KafkaProducerSubcommand::Create(c) => c.name(),
            KafkaProducerSubcommand::Delete(c) => c.name(),
            KafkaProducerSubcommand::List(c) => c.name(),
        }
        .to_string()
    }
}
