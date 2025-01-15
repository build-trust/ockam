use clap::{Args, Subcommand};

use self::revoke::RevokeCommand;
use crate::util::process_nodes_multiaddr;
use crate::{Command, CommandGlobalOpts, Error};
pub use create::CreateCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use miette::IntoDiagnostic;
use std::str::FromStr;

use ockam_api::CliState;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

mod create;
mod list;
mod revoke;
mod show;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct LeaseCommand {
    #[command(subcommand)]
    subcommand: LeaseSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum LeaseSubcommand {
    Create(CreateCommand),
    List(ListCommand),
    Show(ShowCommand),
    Revoke(RevokeCommand),
}

impl LeaseCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            LeaseSubcommand::Create(c) => c.run(ctx, opts).await,
            LeaseSubcommand::List(c) => c.run(ctx, opts).await,
            LeaseSubcommand::Show(c) => c.run(ctx, opts).await,
            LeaseSubcommand::Revoke(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            LeaseSubcommand::Create(c) => c.name(),
            LeaseSubcommand::List(c) => c.name(),
            LeaseSubcommand::Show(c) => c.name(),
            LeaseSubcommand::Revoke(c) => c.name(),
        }
    }
}

fn lease_at_default_value() -> MultiAddr {
    // Backwards compatibility with the service running on the project node
    MultiAddr::from_str("/project/<default_project_name>/service/influxdb_token_lease")
        .expect("Invalid default value for at")
}

async fn resolve_at_arg(at: &MultiAddr, state: &CliState) -> miette::Result<MultiAddr> {
    let mut at = at.to_string();
    if at.contains("<default_project_name>") {
        let project_name = state
            .projects()
            .get_default_project()
            .await
            .map(|p| p.name().to_string())
            .ok()
            .ok_or(Error::arg_validation("at", &at, Some("No projects found")))?;
        at = at.replace("<default_project_name>", &project_name);
    }

    // Parse "to" as a multiaddr again with all the values in place
    let to = MultiAddr::from_str(&at).into_diagnostic()?;
    process_nodes_multiaddr(&to, state).await
}
