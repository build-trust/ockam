mod create;

use clap::{command, Args, Subcommand};

use crate::CommandGlobalOpts;

use self::create::CreateCommand;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct KafkaProducerCommand {
    #[command(subcommand)]
    subcommand: KafkaProducerSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum KafkaProducerSubcommand {
    Create(CreateCommand),
}

impl KafkaProducerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            KafkaProducerSubcommand::Create(c) => c.run(options),
        }
    }
}
