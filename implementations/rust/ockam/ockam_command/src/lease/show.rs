use async_trait::async_trait;
use clap::Args;

use ockam::Context;

use crate::{docs, Command, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

/// Show detailed token information within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct ShowCommand {
    /// ID of the token to retrieve
    #[arg(short, long, value_name = "TOKEN_ID")]
    pub token_id: String,
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "lease show";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        todo!()
    }
}
