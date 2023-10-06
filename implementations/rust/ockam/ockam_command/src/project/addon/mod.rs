mod configure_confluent;
mod configure_influxdb;
mod configure_okta;
mod disable;
mod list;

use core::fmt::Write;

use clap::{Args, Subcommand};
use miette::Context as _;

use ockam_api::cli_state::{CliState, StateDirTrait, StateItemTrait};
use ockam_api::cloud::addon::Addon;
use ockam_api::cloud::project::Projects;
use ockam_api::nodes::InMemoryNode;

use ockam_node::Context;

use crate::project::addon::configure_confluent::AddonConfigureConfluentSubcommand;
use crate::project::addon::configure_influxdb::AddonConfigureInfluxdbSubcommand;
use crate::project::addon::configure_okta::AddonConfigureOktaSubcommand;
use crate::project::addon::disable::AddonDisableSubcommand;
use crate::project::addon::list::AddonListSubcommand;

use crate::output::Output;
use crate::util::api::CloudOpts;

use crate::operation::util::check_for_completion;
use crate::project::util::check_project_readiness;
use crate::{CommandGlobalOpts, Result};

/// Manage addons for a project
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct AddonCommand {
    #[command(subcommand)]
    subcommand: AddonSubcommand,
    #[command(flatten)]
    cloud_opts: CloudOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AddonSubcommand {
    List(AddonListSubcommand),
    Disable(AddonDisableSubcommand),
    #[command(subcommand)]
    Configure(ConfigureAddonCommand),
}

impl AddonCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            AddonSubcommand::List(cmd) => cmd.run(opts),
            AddonSubcommand::Disable(cmd) => cmd.run(opts),
            AddonSubcommand::Configure(cmd) => cmd.run(opts),
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum ConfigureAddonCommand {
    Okta(AddonConfigureOktaSubcommand),
    Influxdb(AddonConfigureInfluxdbSubcommand),
    Confluent(AddonConfigureConfluentSubcommand),
}

impl ConfigureAddonCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self {
            ConfigureAddonCommand::Okta(cmd) => cmd.run(opts),
            ConfigureAddonCommand::Influxdb(cmd) => cmd.run(opts),
            ConfigureAddonCommand::Confluent(cmd) => cmd.run(opts),
        }
    }
}

impl Output for Addon {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "Addon:")?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Enabled: {}", self.enabled)?;
        write!(w, "\n  Description: {}", self.description)?;
        writeln!(w)?;
        Ok(w)
    }
}

impl Output for Vec<Addon> {
    fn output(&self) -> Result<String> {
        if self.is_empty() {
            return Ok("No addons found".to_string());
        }
        let mut w = String::new();
        for (idx, a) in self.iter().enumerate() {
            write!(w, "\n{idx}:")?;
            write!(w, "\n  Id: {}", a.id)?;
            write!(w, "\n  Enabled: {}", a.enabled)?;
            write!(w, "\n  Description: {}", a.description)?;
            writeln!(w)?;
        }
        Ok(w)
    }
}

pub fn get_project_id(cli_state: &CliState, project_name: &str) -> Result<String> {
    Ok(cli_state
        .projects
        .get(project_name)
        .context(format!(
            "Failed to get project {project_name} from config lookup"
        ))?
        .config()
        .id
        .clone())
}

async fn check_configuration_completion(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    project_id: String,
    operation_id: String,
) -> Result<()> {
    let controller = node.create_controller().await?;
    check_for_completion(opts, ctx, &controller, &operation_id).await?;
    let project = controller.get_project(ctx, project_id).await?;
    let _ = check_project_readiness(opts, ctx, node, project).await?;
    Ok(())
}
