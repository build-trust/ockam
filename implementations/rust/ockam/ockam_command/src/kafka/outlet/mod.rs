mod create;

use self::create::CreateCommand;
use crate::CommandGlobalOpts;
use clap::{command, Args, Subcommand};

/// Manage Kafka Outlets
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct KafkaOutletCommand {
    #[command(subcommand)]
    subcommand: KafkaOutletSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum KafkaOutletSubcommand {
    Create(CreateCommand),
}

impl KafkaOutletCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            KafkaOutletSubcommand::Create(c) => c.run(options),
        }
    }
}
