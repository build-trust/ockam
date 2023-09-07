use clap::Args;

use crate::project::addon::configure_kafka::{run_impl, KafkaCommandConfig};
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

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
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, "Instaclustr (Kafka)", self.config));
    }
}
