use clap::Args;

use crate::project::addon::configure_kafka::{AddonConfigureKafkaSubcommand, KafkaCommandConfig};
use crate::{docs, CommandGlobalOpts};

use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("../static/configure_warpstream/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("../static/configure_warpstream/after_long_help.txt");

/// Configure the WarpStream addon for a project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AddonConfigureWarpstreamSubcommand {
    #[command(flatten)]
    config: KafkaCommandConfig,
}

impl AddonConfigureWarpstreamSubcommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        AddonConfigureKafkaSubcommand {
            config: self.config,
        }
        .run(ctx, opts, "Warpstream")
        .await
    }

    pub fn name(&self) -> String {
        "configure warpstream kafka addon".into()
    }
}
