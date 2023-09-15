use clap::Args;
use miette::IntoDiagnostic;
use ockam::Context;
use ockam_api::InfluxDbTokenLease;

use crate::identity::initialize_identity_if_default;
use crate::lease::authenticate;
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::node_rpc;
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
    pub fn run(self, opts: CommandGlobalOpts, cloud_opts: CloudOpts, trust_opts: TrustContextOpts) {
        initialize_identity_if_default(&opts, &cloud_opts.identity);
        node_rpc(run_impl, (opts, cloud_opts, self, trust_opts));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, cmd, trust_opts): (
        CommandGlobalOpts,
        CloudOpts,
        RevokeCommand,
        TrustContextOpts,
    ),
) -> miette::Result<()> {
    let project_node = authenticate(&ctx, &opts, &cloud_opts, &trust_opts).await?;
    project_node
        .revoke_token(&ctx, cmd.token_id.clone())
        .await
        .into_diagnostic()?
        .success()
        .into_diagnostic()?;
    println!("Revoked influxdb token {}.", cmd.token_id);
    Ok(())
}
