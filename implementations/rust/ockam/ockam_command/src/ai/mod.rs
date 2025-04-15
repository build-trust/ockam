mod connect;
mod deploy;
mod enroll;
mod ticket;
pub mod utils;
mod zone_config;

use clap::{Args, Subcommand};

use deploy::DeployCommand;
use enroll::EnrollCommand;
use ockam_node::Context;
use ticket::AiTicketCommand;

use crate::ai::connect::ConnectCommand;
use crate::{docs, Command, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// AI Platform
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AiCommand {
    #[command(subcommand)]
    pub subcommand: AiSubcommand,
}

impl AiCommand {
    pub fn name(&self) -> String {
        self.subcommand.name()
    }

    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            AiSubcommand::Enroll(c) => c.run(ctx, opts).await,
            AiSubcommand::Ticket(c) => c.run(ctx, opts).await,
            AiSubcommand::Deploy(c) => c.run(ctx, opts).await,
            AiSubcommand::Connect(c) => c.run(ctx, opts).await,
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum AiSubcommand {
    Enroll(EnrollCommand),
    Ticket(AiTicketCommand),
    Deploy(DeployCommand),
    Connect(ConnectCommand),
}

impl AiSubcommand {
    pub fn name(&self) -> String {
        match self {
            AiSubcommand::Enroll(c) => c.name(),
            AiSubcommand::Ticket(c) => c.name(),
            AiSubcommand::Deploy(c) => c.name(),
            AiSubcommand::Connect(c) => c.name(),
        }
    }
}
