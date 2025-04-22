mod connect;
mod create;
mod enroll;
mod ticket;
pub mod utils;
mod zone_config;

use clap::{Args, Subcommand};

use create::CreateCommand;
use enroll::EnrollCommand;
use ockam_node::Context;
use ticket::AiTicketCommand;

use crate::cluster::connect::ConnectCommand;
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
pub struct ClusterCommand {
    #[command(subcommand)]
    pub subcommand: ClusterSubcommand,
}

impl ClusterCommand {
    pub fn name(&self) -> String {
        self.subcommand.name()
    }

    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            ClusterSubcommand::Enroll(c) => c.run(ctx, opts).await,
            ClusterSubcommand::Ticket(c) => c.run(ctx, opts).await,
            ClusterSubcommand::Deploy(c) => c.run(ctx, opts).await,
            ClusterSubcommand::Connect(c) => c.run(ctx, opts).await,
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ClusterSubcommand {
    Enroll(EnrollCommand),
    Ticket(AiTicketCommand),
    Deploy(CreateCommand),
    Connect(ConnectCommand),
}

impl ClusterSubcommand {
    pub fn name(&self) -> String {
        match self {
            ClusterSubcommand::Enroll(c) => c.name(),
            ClusterSubcommand::Ticket(c) => c.name(),
            ClusterSubcommand::Deploy(c) => c.name(),
            ClusterSubcommand::Connect(c) => c.name(),
        }
    }
}
