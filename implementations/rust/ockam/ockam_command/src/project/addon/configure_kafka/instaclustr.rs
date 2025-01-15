use clap::Args;

use crate::project::addon::configure_kafka::{AddonConfigureKafkaSubcommand, KafkaCommandConfig};
use crate::{docs, CommandGlobalOpts};

use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("../static/configure_instaclustr/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("../static/configure_instaclustr/after_long_help.txt");

/// Configure the Instaclustr addon for a project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AddonConfigureInstaclustrSubcommand {
    #[command(flatten)]
    config: KafkaCommandConfig,
}

impl AddonConfigureInstaclustrSubcommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        AddonConfigureKafkaSubcommand {
            config: self.config,
        }
        .run(ctx, opts, "Instaclustr (Kafka)")
        .await
    }

    pub fn name(&self) -> String {
        "configure instaclustr kafka addon".into()
    }
}
