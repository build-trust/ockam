use clap::{command, Args, Subcommand};

use crate::kafka::direct::create::CreateCommand;
use crate::kafka::direct::delete::DeleteCommand;
use crate::kafka::direct::list::ListCommand;
use crate::CommandGlobalOpts;

mod create;
mod delete;
mod list;
mod rpc;

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
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            KafkaDirectSubcommand::Create(c) => c.run(options),
            KafkaDirectSubcommand::Delete(c) => c.run(options),
            KafkaDirectSubcommand::List(c) => c.run(options),
        }
    }
}
