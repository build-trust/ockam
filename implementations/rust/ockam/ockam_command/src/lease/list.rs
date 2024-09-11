use async_trait::async_trait;
use clap::Args;
use ockam::Context;

use crate::{docs, Command, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

/// List tokens within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct ListCommand;

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "lease list";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        todo!()
    }
}
