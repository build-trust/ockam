pub(crate) mod common_args;
pub(crate) mod create;
pub(crate) mod ctrlc;
pub mod delete;
pub(crate) mod deploy;
pub mod get_cluster_name;
pub(crate) mod init;
pub(crate) mod inlet;
pub mod list;
mod outlet;
pub(crate) mod repl;
pub(crate) mod secret;
pub mod zone_config;

use clap::{Args, Subcommand};

use create::CreateCommand;
use deploy::DeployCommand;
use init::InitCommand;
use ockam_node::Context;

use crate::zone::delete::DeleteCommand;
use crate::zone::inlet::InletCommand;
use crate::zone::list::ListCommand;
use crate::zone::outlet::OutletCommand;
use crate::zone::repl::ReplCommand;
use crate::zone::secret::SecretCommand;
use crate::{docs, Command, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Manage Zones
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ZoneCommand {
    #[command(subcommand)]
    pub subcommand: ZoneSubcommand,
}

impl ZoneCommand {
    pub fn name(&self) -> String {
        self.subcommand.name()
    }

    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            ZoneSubcommand::Init(c) => c.run(ctx, opts).await.map(|_| ()),
            ZoneSubcommand::Secret(c) => c.run(ctx, opts).await,
            ZoneSubcommand::Create(c) => c.run(ctx, opts).await.map(|_| ()),
            ZoneSubcommand::Deploy(c) => c.run(ctx, opts).await.map(|_| ()),
            ZoneSubcommand::List(c) => c.run(ctx, opts).await,
            ZoneSubcommand::Delete(c) => c.run(ctx, opts).await,
            ZoneSubcommand::Inlet(c) => c.run(ctx, opts).await,
            ZoneSubcommand::Outlet(c) => c.run(ctx, opts).await,
            ZoneSubcommand::Repl(c) => c.run(ctx, opts).await.map(|_| ()),
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ZoneSubcommand {
    Init(InitCommand),
    Secret(SecretCommand),
    #[command(hide = true)]
    Create(CreateCommand),
    Deploy(DeployCommand),
    List(ListCommand),
    Delete(DeleteCommand),
    Inlet(InletCommand),
    Outlet(OutletCommand),
    #[command(hide = true)]
    Repl(ReplCommand),
}

impl ZoneSubcommand {
    pub fn name(&self) -> String {
        match self {
            ZoneSubcommand::Init(c) => c.name(),
            ZoneSubcommand::Secret(c) => c.name(),
            ZoneSubcommand::Create(c) => c.name(),
            ZoneSubcommand::Deploy(c) => c.name(),
            ZoneSubcommand::List(c) => c.name(),
            ZoneSubcommand::Delete(c) => c.name(),
            ZoneSubcommand::Inlet(c) => c.name(),
            ZoneSubcommand::Outlet(c) => c.name(),
            ZoneSubcommand::Repl(c) => c.name(),
        }
    }
}
