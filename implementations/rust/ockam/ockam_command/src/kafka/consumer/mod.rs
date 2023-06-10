use clap::{command, Args, Subcommand};

use crate::kafka::consumer::create::CreateCommand;
use crate::kafka::consumer::delete::DeleteCommand;
use crate::kafka::consumer::list::ListCommand;
use crate::CommandGlobalOpts;

mod create;
mod delete;
mod list;

/// Manage Kafka Consumers
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct KafkaConsumerCommand {
    #[command(subcommand)]
    subcommand: KafkaConsumerSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum KafkaConsumerSubcommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl KafkaConsumerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            KafkaConsumerSubcommand::Create(c) => c.run(options),
            KafkaConsumerSubcommand::Delete(c) => c.run(options),
            KafkaConsumerSubcommand::List(c) => c.run(options),
        }
    }
}
