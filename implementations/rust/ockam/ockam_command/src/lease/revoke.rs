use clap::Args;
use ockam::Context;
use ockam_api::InfluxDbTokenLease;

use crate::lease::create_project_client;
use crate::util::api::{CloudOpts, TrustOpts};
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

/// Revoke a token within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct RevokeCommand {
    /// ID of the token to revoke
    #[arg(long, short, id = "token_id", value_name = "TOKEN_ID")]
    pub token_id: String,
}

impl RevokeCommand {
    pub fn run(
        self,
        opts: CommandGlobalOpts,
        cloud_opts: CloudOpts,
        trust_opts: TrustOpts,
    ) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts, cloud_opts, trust_opts).await
        })
    }

    pub fn name(&self) -> String {
        "revoke token".into()
    }

    async fn async_run(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        cloud_opts: CloudOpts,
        trust_opts: TrustOpts,
    ) -> miette::Result<()> {
        let project_node = create_project_client(ctx, &opts, &cloud_opts, &trust_opts).await?;
        project_node
            .revoke_token(ctx, self.token_id.clone())
            .await?;
        println!("Revoked influxdb token {}.", self.token_id);
        Ok(())
    }
}
