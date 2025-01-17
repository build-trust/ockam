use clap::{Args, Subcommand};

use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;
use ockam_node::Context;

use crate::operation::util::check_for_operation_completion;
use crate::project::addon::configure_influxdb::AddonConfigureInfluxdbSubcommand;
use crate::project::addon::configure_kafka::{
    AddonConfigureAivenSubcommand, AddonConfigureConfluentSubcommand,
    AddonConfigureInstaclustrSubcommand, AddonConfigureKafkaSubcommand,
    AddonConfigureRedpandaSubcommand, AddonConfigureWarpstreamSubcommand,
};
use crate::project::addon::configure_okta::AddonConfigureOktaSubcommand;
use crate::project::addon::disable::AddonDisableSubcommand;
use crate::project::addon::list::AddonListSubcommand;
use crate::project::util::check_project_readiness;
use crate::shared_args::IdentityOpts;
use crate::{CommandGlobalOpts, Result};

mod configure_influxdb;
mod configure_kafka;
mod configure_okta;
mod disable;
mod list;

/// Manage addons for a Project
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct AddonCommand {
    #[command(subcommand)]
    subcommand: AddonSubcommand,
    #[command(flatten)]
    identity_opts: IdentityOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AddonSubcommand {
    List(AddonListSubcommand),
    Disable(AddonDisableSubcommand),
    #[command(subcommand)]
    Configure(ConfigureAddonCommand),
}

impl AddonCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            AddonSubcommand::List(cmd) => cmd.run(opts),
            AddonSubcommand::Disable(cmd) => cmd.run(opts),
            AddonSubcommand::Configure(cmd) => cmd.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            AddonSubcommand::List(c) => c.name(),
            AddonSubcommand::Disable(c) => c.name(),
            AddonSubcommand::Configure(c) => c.name(),
        }
    }
}

/// Configure an addon for a project
#[derive(Clone, Debug, Subcommand)]
pub enum ConfigureAddonCommand {
    Okta(AddonConfigureOktaSubcommand),
    Influxdb(AddonConfigureInfluxdbSubcommand),
    Confluent(AddonConfigureConfluentSubcommand),
    InstaclustrKafka(AddonConfigureInstaclustrSubcommand),
    AivenKafka(AddonConfigureAivenSubcommand),
    Redpanda(AddonConfigureRedpandaSubcommand),
    Warpstream(AddonConfigureWarpstreamSubcommand),
    Kafka(AddonConfigureKafkaSubcommand),
}

impl ConfigureAddonCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self {
            ConfigureAddonCommand::Okta(cmd) => cmd.run(opts),
            ConfigureAddonCommand::Influxdb(cmd) => cmd.run(opts),
            ConfigureAddonCommand::Confluent(cmd) => cmd.run(opts),
            ConfigureAddonCommand::InstaclustrKafka(cmd) => cmd.run(opts),
            ConfigureAddonCommand::AivenKafka(cmd) => cmd.run(opts),
            ConfigureAddonCommand::Redpanda(cmd) => cmd.run(opts),
            ConfigureAddonCommand::Warpstream(cmd) => cmd.run(opts),
            ConfigureAddonCommand::Kafka(cmd) => cmd.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self {
            ConfigureAddonCommand::Okta(c) => c.name(),
            ConfigureAddonCommand::Influxdb(c) => c.name(),
            ConfigureAddonCommand::Confluent(c) => c.name(),
            ConfigureAddonCommand::InstaclustrKafka(c) => c.name(),
            ConfigureAddonCommand::AivenKafka(c) => c.name(),
            ConfigureAddonCommand::Redpanda(c) => c.name(),
            ConfigureAddonCommand::Warpstream(c) => c.name(),
            ConfigureAddonCommand::Kafka(c) => c.name(),
        }
    }
}

async fn check_configuration_completion(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    project_id: &str,
    operation_id: &str,
) -> Result<()> {
    check_for_operation_completion(opts, ctx, node, operation_id, "the addon configuration")
        .await?;
    let project = node.get_project(ctx, project_id).await?;
    let _ = check_project_readiness(opts, ctx, node, project).await?;
    Ok(())
}
