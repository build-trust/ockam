use async_trait::async_trait;

use clap::Args;
use ockam_node::Context;

use crate::{docs, Command, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/deploy/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/deploy/after_long_help.txt");

/// Deploy an Ockam AI Agent into a Zone
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeployCommand {}

#[async_trait]
impl Command for DeployCommand {
    const NAME: &'static str = "ai deploy";

    async fn run(self, _ctx: &Context, _opts: CommandGlobalOpts) -> Result<()> {
        todo!()
    }
}
