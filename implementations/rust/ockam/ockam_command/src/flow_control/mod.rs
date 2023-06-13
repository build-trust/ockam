use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

mod add_consumer;

pub use add_consumer::AddConsumerCommand;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct FlowControlCommand {
    #[command(subcommand)]
    subcommand: FlowControlSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum FlowControlSubcommand {
    #[command(display_order = 800)]
    AddConsumer(AddConsumerCommand),
}

impl FlowControlCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            FlowControlSubcommand::AddConsumer(c) => c.run(options),
        }
    }
}
