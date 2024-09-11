use async_trait::async_trait;
use clap::Args;

use ockam::Context;

use crate::{docs, Command, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

/// Revoke a token within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct RevokeCommand {
    /// ID of the token to revoke
    #[arg(long, short, id = "token_id", value_name = "TOKEN_ID")]
    pub token_id: String,
}

#[async_trait]
impl Command for RevokeCommand {
    const NAME: &'static str = "lease revoke";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        todo!()
    }
}
