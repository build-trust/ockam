use clap::Args;
use ockam::Context;
use ockam_api::InfluxDbTokenLease;

use crate::lease::create_project_client;
use crate::util::api::{CloudOpts, TrustOpts};
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
    pub fn run(self, opts: CommandGlobalOpts, cloud_opts: CloudOpts, trust_opts: TrustOpts) {
        node_rpc(
            opts.rt.clone(),
            run_impl,
            (opts, cloud_opts, self, trust_opts),
        );
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, cmd, trust_opts): (CommandGlobalOpts, CloudOpts, RevokeCommand, TrustOpts),
) -> miette::Result<()> {
    let project_node_client = create_project_client(&ctx, &opts, &cloud_opts, &trust_opts).await?;
    project_node_client
        .revoke_token(&ctx, cmd.token_id.clone())
        .await?;
    println!("Revoked influxdb token {}.", cmd.token_id);
    Ok(())
}
