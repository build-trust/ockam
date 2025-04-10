use async_trait::async_trait;
use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;
use std::sync::Arc;

pub use aiven::AddonConfigureAivenSubcommand;
pub use confluent::AddonConfigureConfluentSubcommand;
pub use instaclustr::AddonConfigureInstaclustrSubcommand;
use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::addon::{Addons, KafkaConfig};
pub use redpanda::AddonConfigureRedpandaSubcommand;
pub use warpstream::AddonConfigureWarpstreamSubcommand;

use crate::node_command::InMemoryNodeCommand;
use crate::project::addon::check_configuration_completion;
use crate::{docs, CommandGlobalOpts};

pub mod aiven;
pub mod confluent;
pub mod instaclustr;
pub mod redpanda;
pub mod warpstream;

const LONG_ABOUT: &str = include_str!("./static/configure_kafka/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/configure_kafka/after_long_help.txt");

/// Configure the Apache Kafka addon for a project
#[derive(Clone, Debug, Args)]
pub struct KafkaCommandConfig {
    #[arg(
        help = docs::about("Ockam project name"),
        long = "project",
        id = "project",
        value_name = "PROJECT_NAME",
        default_value = "default",
        value_parser(NonEmptyStringValueParser::new())
    )]
    project_name: String,

    /// Bootstrap server address
    #[arg(
        long,
        id = "bootstrap_server",
        value_name = "BOOTSTRAP_SERVER",
        value_parser(NonEmptyStringValueParser::new())
    )]
    bootstrap_server: String,
}

#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AddonConfigureKafkaSubcommand {
    #[command(flatten)]
    config: KafkaCommandConfig,
}

#[derive(Clone)]
struct AddonConfigureKafkaNodeCommand {
    opts: CommandGlobalOpts,
    command: AddonConfigureKafkaSubcommand,
    addon_name: String,
}

impl AddonConfigureKafkaNodeCommand {
    pub fn new(
        opts: CommandGlobalOpts,
        command: AddonConfigureKafkaSubcommand,
        addon_name: &str,
    ) -> Self {
        Self {
            opts,
            command,
            addon_name: addon_name.to_string(),
        }
    }
}

#[async_trait]
impl InMemoryNodeCommand for AddonConfigureKafkaNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let project_id = self
            .opts
            .state
            .projects()
            .get_project_by_name(&self.command.config.project_name.clone())
            .await?
            .project_id()
            .to_string();
        let config = KafkaConfig::new(self.command.config.bootstrap_server.clone());

        let controller = node.create_controller().await?;

        let response = controller
            .configure_confluent_addon(node.ctx(), &project_id, config)
            .await?;
        check_configuration_completion(&self.opts, &node, &project_id, &response.operation_id)
            .await?;

        self.opts
            .terminal
            .write_line(fmt_ok!("{} addon configured successfully", self.addon_name))?;

        Ok(())
    }
}

impl AddonConfigureKafkaSubcommand {
    pub fn name(&self) -> String {
        "configure kafka addon".into()
    }

    pub async fn run(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        addon_name: &str,
    ) -> miette::Result<()> {
        AddonConfigureKafkaNodeCommand::new(opts.clone(), self.clone(), addon_name)
            .execute(ctx, opts.state)
            .await
    }
}
