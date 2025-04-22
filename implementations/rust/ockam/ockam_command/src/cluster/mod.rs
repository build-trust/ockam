mod create;
mod delete;
mod enroll;
mod init;
mod inlet;
mod outlet;
mod ticket;
pub mod utils;
mod zone_config;

use clap::{Args, Subcommand};

use create::CreateCommand;
use enroll::EnrollCommand;
use ockam_node::Context;
use ticket::AiTicketCommand;

use crate::cluster::delete::DeleteCommand;
use crate::cluster::init::InitCommand;
use crate::cluster::inlet::InletCommand;
use crate::cluster::outlet::OutletCommand;
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
            ClusterSubcommand::Init(c) => c.run(ctx, opts).await,
            ClusterSubcommand::Enroll(c) => c.run(ctx, opts).await,
            ClusterSubcommand::Ticket(c) => c.run(ctx, opts).await,
            ClusterSubcommand::Create(c) => c.run(ctx, opts).await,
            ClusterSubcommand::Delete(c) => c.run(ctx, opts).await,
            ClusterSubcommand::Inlet(c) => c.run(ctx, opts).await,
            ClusterSubcommand::Outlet(c) => c.run(ctx, opts).await,
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ClusterSubcommand {
    Init(InitCommand),
    Enroll(EnrollCommand),
    Ticket(AiTicketCommand),
    Create(CreateCommand),
    Delete(DeleteCommand),
    Inlet(InletCommand),
    Outlet(OutletCommand),
}

impl ClusterSubcommand {
    pub fn name(&self) -> String {
        match self {
            ClusterSubcommand::Init(c) => c.name(),
            ClusterSubcommand::Enroll(c) => c.name(),
            ClusterSubcommand::Ticket(c) => c.name(),
            ClusterSubcommand::Create(c) => c.name(),
            ClusterSubcommand::Delete(c) => c.name(),
            ClusterSubcommand::Inlet(c) => c.name(),
            ClusterSubcommand::Outlet(c) => c.name(),
        }
    }
}
