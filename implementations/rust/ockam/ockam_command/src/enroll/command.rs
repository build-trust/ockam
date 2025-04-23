use async_trait::async_trait;
use clap::Args;

use crate::enroll::handler::EnrollHandler;
use crate::{docs, Command, CommandGlobalOpts};
use ockam::Context;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

#[derive(Clone, Debug, Args, Default)]
#[command(
about = docs::about("Enroll your Ockam Identity with Ockam Orchestrator"),
long_about = docs::about(LONG_ABOUT),
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

    /// By default this command skips the enrollment process if the Identity you specified
    /// (using `--identity`), or the default Identity, is already enrolled, by checking
    /// its status. Use this flag to force the execution of the Identity enrollment
    /// process.
    #[arg(long)]
    pub force: bool,

    /// Use this flag to skip creating Orchestrator resources. When you use this flag, we
    /// only check whether the Orchestrator resources are created. And if they are not, we
    /// will continue without creating them.
    #[arg(hide = true, long = "skip-resource-creation", conflicts_with = "force")]
    pub skip_orchestrator_resources_creation: bool,
}

#[async_trait]
impl Command for EnrollCommand {
    const NAME: &'static str = "enroll";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let handler = EnrollHandler {
            opts: opts.clone(),
            identity_name: self.identity.clone(),
            authorization_code_flow: self.authorization_code_flow,
            force: self.force,
            skip_orchestrator_resources_creation: self.skip_orchestrator_resources_creation,
            is_ai_cloud_account: false,
        };
        handler.handle(ctx).await?;
        Ok(())
    }
}
