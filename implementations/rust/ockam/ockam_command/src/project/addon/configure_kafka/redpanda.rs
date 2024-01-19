use clap::Args;

use crate::project::addon::configure_kafka::{AddonConfigureKafkaSubcommand, KafkaCommandConfig};
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("../static/configure_redpanda/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("../static/configure_redpanda/after_long_help.txt");

/// Configure the Redpanda addon for a project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AddonConfigureRedpandaSubcommand {
    #[command(flatten)]
    config: KafkaCommandConfig,
}

impl AddonConfigureRedpandaSubcommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            AddonConfigureKafkaSubcommand {
                config: self.config,
            }
            .async_run(&ctx, opts, "Redpanda")
            .await
        })
    }

    pub fn name(&self) -> String {
        "configure redpanda kafka addon".into()
    }
}
