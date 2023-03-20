mod configure_confluent;
mod configure_influxdb;
mod configure_okta;
mod disable;
mod list;

use core::fmt::Write;

use anyhow::Context as _;

use clap::{Args, Subcommand};

use ockam_api::cloud::addon::Addon;

use ockam_api::config::lookup::ConfigLookup;

use crate::project::addon::configure_confluent::AddonConfigureConfluentSubcommand;
use crate::project::addon::configure_influxdb::AddonConfigureInfluxdbSubcommand;
use crate::project::addon::configure_okta::AddonConfigureOktaSubcommand;
use crate::project::addon::disable::AddonDisableSubcommand;
use crate::project::addon::list::AddonListSubcommand;

use crate::util::api::CloudOpts;
use crate::util::output::Output;

use crate::{CommandGlobalOpts, Result};

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
            AddonSubcommand::List(cmd) => cmd.run(opts, self.cloud_opts),
            AddonSubcommand::Disable(cmd) => cmd.run(opts, self.cloud_opts),
            AddonSubcommand::Configure(cmd) => cmd.run(opts, self.cloud_opts),
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
    pub fn run(self, opts: CommandGlobalOpts, cloud_opts: CloudOpts) {
        match self {
            ConfigureAddonCommand::Okta(cmd) => cmd.run(opts, cloud_opts),
            ConfigureAddonCommand::Influxdb(cmd) => cmd.run(opts, cloud_opts),
            ConfigureAddonCommand::Confluent(cmd) => cmd.run(opts, cloud_opts),
        }
    }
}

impl Output for Addon<'_> {
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

impl Output for Vec<Addon<'_>> {
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

pub fn base_endpoint(lookup: &ConfigLookup, project_name: &str) -> Result<String> {
    let project_id = &lookup
        .get_project(project_name)
        .context(format!(
            "Failed to get project {project_name} from config lookup"
        ))?
        .id;
    Ok(format!("{project_id}/addons"))
}
