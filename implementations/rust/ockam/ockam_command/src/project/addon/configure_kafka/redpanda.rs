use clap::Args;

use crate::project::addon::configure_kafka::{run_impl, KafkaCommandConfig};
use crate::util::node_rpc;
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
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(opts.rt.clone(), run_impl, (opts, "Redpanda", self.config));
    }
}
