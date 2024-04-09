use clap::{command, Args, Subcommand};

use crate::CommandGlobalOpts;

use self::create::CreateCommand;

mod create;

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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            KafkaOutletSubcommand::Create(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            KafkaOutletSubcommand::Create(c) => c.name(),
        }
        .to_string()
    }
}
