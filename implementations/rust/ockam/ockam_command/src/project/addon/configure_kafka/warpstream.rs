use clap::Args;

use crate::project::addon::configure_kafka::{AddonConfigureKafkaSubcommand, KafkaCommandConfig};
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            AddonConfigureKafkaSubcommand {
                config: self.config,
            }
            .async_run(&ctx, opts, "Warpstream")
            .await
        })
    }

    pub fn name(&self) -> String {
        "configure warpstream kafka addon".into()
    }
}
