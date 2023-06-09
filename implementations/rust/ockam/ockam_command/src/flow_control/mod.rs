use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

mod add_consumer_for_producer;
mod add_consumer_for_spawner;

pub use add_consumer_for_producer::AddConsumerForProducerCommand;
pub use add_consumer_for_spawner::AddConsumerForSpawnerCommand;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct FlowControlCommand {
    #[command(subcommand)]
    subcommand: FlowControlSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum FlowControlSubcommand {
    #[command(display_order = 800)]
    AddConsumerForSpawner(AddConsumerForSpawnerCommand),
    #[command(display_order = 800)]
    AddConsumerForProducer(AddConsumerForProducerCommand),
}

impl FlowControlCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            FlowControlSubcommand::AddConsumerForSpawner(c) => c.run(options),
            FlowControlSubcommand::AddConsumerForProducer(c) => c.run(options),
        }
    }
}
