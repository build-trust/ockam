pub mod aiven;
pub mod confluent;
pub mod instaclustr;
pub mod redpanda;
pub mod warpstream;

use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cloud::addon::{Addons, KafkaConfig};
use ockam_api::nodes::InMemoryNode;

use crate::project::addon::check_configuration_completion;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

pub use aiven::AddonConfigureAivenSubcommand;
pub use confluent::AddonConfigureConfluentSubcommand;
pub use instaclustr::AddonConfigureInstaclustrSubcommand;
pub use redpanda::AddonConfigureRedpandaSubcommand;
pub use warpstream::AddonConfigureWarpstreamSubcommand;

const LONG_ABOUT: &str = include_str!("./static/configure_kafka/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/configure_kafka/after_long_help.txt");

/// Configure the Apache Kafka addon for a project
#[derive(Clone, Debug, Args)]
pub struct KafkaCommandConfig {
    /// Ockam project name
    #[arg(
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
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, "Apache Kafka", self.config));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, addon_name, cmd): (CommandGlobalOpts, &str, KafkaCommandConfig),
) -> miette::Result<()> {
    let KafkaCommandConfig {
        project_name,
        bootstrap_server,
    } = cmd;
    let project_id = &opts.state.get_project_by_name(&project_name).await?.id();
    let config = KafkaConfig::new(bootstrap_server);

    let node = InMemoryNode::start(&ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    let response = controller
        .configure_confluent_addon(&ctx, project_id, config)
        .await?;
    check_configuration_completion(&opts, &ctx, &node, project_id, &response.operation_id).await?;

    opts.terminal
        .write_line(&fmt_ok!("{} addon configured successfully", addon_name))?;

    Ok(())
}
