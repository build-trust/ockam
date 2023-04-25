mod create;

use clap::{command, Args, Subcommand};

use crate::CommandGlobalOpts;

use self::create::CreateCommand;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct KafkaConsumerCommand {
    #[command(subcommand)]
    subcommand: KafkaConsumerSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum KafkaConsumerSubcommand {
    Create(CreateCommand),
}

impl KafkaConsumerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            KafkaConsumerSubcommand::Create(c) => c.run(options),
        }
    }
}
