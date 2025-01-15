use clap::{command, Args, Subcommand};

use crate::kafka::inlet::create::CreateCommand;
use crate::kafka::inlet::delete::DeleteCommand;
use crate::kafka::inlet::list::ListCommand;
use crate::kafka::inlet::show::ShowCommand;
use crate::{Command, CommandGlobalOpts};

use ockam_node::Context;

pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod list;
pub(crate) mod show;

/// Manage Kafka Inlets
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct KafkaInletCommand {
    #[command(subcommand)]
    pub(crate) subcommand: KafkaInletSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum KafkaInletSubcommand {
    Create(CreateCommand),
    Show(ShowCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl KafkaInletCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            KafkaInletSubcommand::Create(c) => c.run(ctx, opts).await,
            KafkaInletSubcommand::Show(c) => c.run(ctx, opts).await,
            KafkaInletSubcommand::Delete(c) => c.run(ctx, opts).await,
            KafkaInletSubcommand::List(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            KafkaInletSubcommand::Create(c) => c.name(),
            KafkaInletSubcommand::Show(c) => c.name(),
            KafkaInletSubcommand::Delete(c) => c.name(),
            KafkaInletSubcommand::List(c) => c.name(),
        }
    }
}
