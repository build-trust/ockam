use clap::Args;

use crate::node::NodeOpts;
use crate::util::print_warning_for_deprecated_flag_replaced;
use crate::{docs, Command, CommandGlobalOpts};

use ockam_node::Context;

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List Kafka Producers.
/// [DEPRECATED]
#[derive(Args, Clone, Debug)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        print_warning_for_deprecated_flag_replaced(&opts, &self.name(), "kafka-inlet")?;
        crate::kafka::inlet::list::ListCommand {
            node_opts: self.node_opts,
        }
        .run(ctx, opts)
        .await
    }

    pub fn name(&self) -> String {
        "list kafka producers".into()
    }
}
