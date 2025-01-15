use clap::{command, Args, Subcommand};

use crate::kafka::consumer::create::CreateCommand;
use crate::kafka::consumer::delete::DeleteCommand;
use crate::kafka::consumer::list::ListCommand;
use crate::CommandGlobalOpts;

use ockam_node::Context;

mod create;
mod delete;
mod list;

/// Manage Kafka Consumers
/// [DEPRECATED]
#[derive(Clone, Debug, Args)]
#[command(hide = true, arg_required_else_help = true, subcommand_required = true)]
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
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            KafkaConsumerSubcommand::Create(c) => c.run(ctx, opts).await,
            KafkaConsumerSubcommand::Delete(c) => c.run(ctx, opts).await,
            KafkaConsumerSubcommand::List(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            KafkaConsumerSubcommand::Create(c) => c.name(),
            KafkaConsumerSubcommand::Delete(c) => c.name(),
            KafkaConsumerSubcommand::List(c) => c.name(),
        }
    }
}
