pub(crate) mod config;

pub(crate) mod list;
pub(crate) mod start;

use clap::{Args, Subcommand};

use crate::{docs, CommandGlobalOpts};
use list::ListCommand;
pub(crate) use start::StartCommand;

use ockam_node::Context;

#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide())]
pub struct ServiceCommand {
    #[command(subcommand)]
    subcommand: ServiceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ServiceSubcommand {
    #[command(display_order = 900)]
    Start(StartCommand),
    #[command(display_order = 901)]
    List(ListCommand),
}

impl ServiceCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            ServiceSubcommand::Start(c) => c.run(ctx, opts).await,
            ServiceSubcommand::List(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            ServiceSubcommand::Start(c) => c.name(),
            ServiceSubcommand::List(c) => c.name(),
        }
    }
}
