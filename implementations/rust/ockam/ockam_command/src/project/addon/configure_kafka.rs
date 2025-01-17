use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;

pub use aiven::AddonConfigureAivenSubcommand;
pub use confluent::AddonConfigureConfluentSubcommand;
pub use instaclustr::AddonConfigureInstaclustrSubcommand;
use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::addon::{Addons, KafkaConfig};
pub use redpanda::AddonConfigureRedpandaSubcommand;
pub use warpstream::AddonConfigureWarpstreamSubcommand;

use crate::project::addon::check_configuration_completion;
use crate::util::async_cmd;
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

impl AddonConfigureKafkaSubcommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts, "Apache Kafka").await
        })
    }

    pub fn name(&self) -> String {
        "configure kafka addon".into()
    }

    async fn async_run(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        addon_name: &str,
    ) -> miette::Result<()> {
        let project_id = opts
            .state
            .projects()
            .get_project_by_name(&self.config.project_name.clone())
            .await?
            .project_id()
            .to_string();
        let config = KafkaConfig::new(self.config.bootstrap_server.clone());

        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        let response = controller
            .configure_confluent_addon(ctx, &project_id, config)
            .await?;
        check_configuration_completion(&opts, ctx, &node, &project_id, &response.operation_id)
            .await?;

        opts.terminal
            .write_line(fmt_ok!("{} addon configured successfully", addon_name))?;

        Ok(())
    }
}
