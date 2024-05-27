pub(crate) mod create;
mod delete;
mod list;

use self::create::CreateCommand;
use self::delete::DeleteCommand;
use self::list::ListCommand;
use crate::{Command, CommandGlobalOpts};
use clap::{command, Args, Subcommand};

/// Manage Kafka Outlets
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct KafkaOutletCommand {
    #[command(subcommand)]
    pub(crate) subcommand: KafkaOutletSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum KafkaOutletSubcommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl KafkaOutletCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            KafkaOutletSubcommand::Create(c) => c.run(opts),
            KafkaOutletSubcommand::Delete(c) => c.run(opts),
            KafkaOutletSubcommand::List(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            KafkaOutletSubcommand::Create(c) => c.name(),
            KafkaOutletSubcommand::Delete(c) => c.name(),
            KafkaOutletSubcommand::List(c) => c.name(),
        }
        .to_string()
    }
}
