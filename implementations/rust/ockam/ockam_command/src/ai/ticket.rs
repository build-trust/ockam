use async_trait::async_trait;

use clap::Args;
use ockam_node::Context;

use crate::{docs, Command, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/ticket/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/ticket/after_long_help.txt");

/// Generate an enrollment ticket for an Ockam AI Agent
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct AiTicketCommand {
    #[command(flatten)]
    inner: crate::project::TicketCommand,
}

#[async_trait]
impl Command for AiTicketCommand {
    const NAME: &'static str = "ai ticket";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        self.inner.run(ctx, opts).await
    }
}
