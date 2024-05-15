use clap::Args;

use crate::util::print_deprecated_warning;
use crate::{docs, node::NodeOpts, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Kafka Consumer
/// [DEPRECATED]
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Kafka consumer service address
    pub address: String,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        print_deprecated_warning(&opts, &self.name(), "kafka-inlet")?;
        crate::kafka::inlet::delete::DeleteCommand {
            node_opts: self.node_opts,
            address: self.address,
        }
        .run(opts)
    }

    pub fn name(&self) -> String {
        "delete kafka consumer".into()
    }
}
