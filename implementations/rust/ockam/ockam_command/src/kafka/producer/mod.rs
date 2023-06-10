use clap::{command, Args, Subcommand};

use crate::kafka::producer::create::CreateCommand;
use crate::kafka::producer::delete::DeleteCommand;
use crate::kafka::producer::list::ListCommand;
use crate::CommandGlobalOpts;

mod create;
mod delete;
mod list;

/// Manage Kafka Producers
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
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
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            KafkaProducerSubcommand::Create(c) => c.run(options),
            KafkaProducerSubcommand::Delete(c) => c.run(options),
            KafkaProducerSubcommand::List(c) => c.run(options),
        }
    }
}
