use async_trait::async_trait;
use clap::Args;

use crate::enroll::handler::EnrollHandler;
use crate::{docs, Command, CommandGlobalOpts};
use ockam::Context;

const LONG_ABOUT: &str = include_str!("./static/enroll/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/enroll/after_long_help.txt");

/// Enroll your Ockam Identity with Ockam Orchestrator
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct EnrollCommand {
    #[arg(global = true, value_name = "IDENTITY_NAME", long)]
    #[arg(help = docs::about("\
    The name of an existing Ockam Identity that you wish to enroll. \
    You can use `ockam identity list` to get a list of existing Identities. \
    To create a new Identity, use `ockam identity create`. \
    If you don't specify an Identity name, and you don't have a default Identity, this command \
    will create a default Identity for you and save it locally in the default Vault
    "))]
    pub identity: Option<String>,

    /// This option allows you to bypass pasting the one-time code and confirming device
    /// activation, and PKCE (Proof Key for Code Exchange) authorization flow. Please be
    /// careful with this option since it will open your default system browser. This
    /// option might be useful if you have already enrolled and want to re-enroll using
    /// the same account information
    #[arg(long)]
    pub authorization_code_flow: bool,
}

#[async_trait]
impl Command for EnrollCommand {
    const NAME: &'static str = "cluster enroll";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let handler = EnrollHandler {
            opts: opts.clone(),
            identity_name: self.identity.clone(),
            authorization_code_flow: self.authorization_code_flow,
            force: false,
            skip_orchestrator_resources_creation: true,
            is_ai_cloud_account: true,
        };
        handler.handle(ctx).await?;
        Ok(())
    }
}
