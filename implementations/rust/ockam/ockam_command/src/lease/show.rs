use clap::Args;

use ockam::Context;
use ockam_api::InfluxDbTokenLeaseManagerTrait;

use crate::lease::create_project_client;
use crate::shared_args::{IdentityOpts, TrustOpts};
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};
use ockam_api::output::Output;

const HELP_DETAIL: &str = "";

/// Show detailed token information within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct ShowCommand {
    /// ID of the token to retrieve
    #[arg(short, long, value_name = "TOKEN_ID")]
    pub token_id: String,
}

impl ShowCommand {
    pub fn run(
        self,
        opts: CommandGlobalOpts,
        identity_opts: IdentityOpts,
        trust_opts: TrustOpts,
    ) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts, identity_opts, trust_opts).await
        })
    }

    pub fn name(&self) -> String {
        "lease show".into()
    }

    async fn async_run(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        identity_opts: IdentityOpts,
        trust_opts: TrustOpts,
    ) -> miette::Result<()> {
        let project_node = create_project_client(ctx, &opts, &identity_opts, &trust_opts).await?;
        let token = project_node.get_token(ctx, self.token_id.clone()).await?;

        opts.terminal
            .stdout()
            .plain(token.item()?)
            .json(serde_json::json!(&token))
            .write_line()?;

        Ok(())
    }
}
