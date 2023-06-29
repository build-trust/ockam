use std::str::FromStr;

use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::{
    CommandGlobalOpts,
    docs,
    util::{
        api::{CloudOpts, TrustContextOpts},
        node_rpc,
        orchestrator_api::OrchestratorApiBuilder,
    },
};
use crate::identity::{get_identity_name, initialize_identity_if_default};

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
    let identity = get_identity_name(&opts.state, &cloud_opts.identity);
    let mut orchestrator_client = OrchestratorApiBuilder::new(&ctx, &opts, &trust_opts)
        .as_identity(identity)
        .with_new_embbeded_node()
        .await?
        .build(&MultiAddr::from_str("/service/influxdb_token_lease").into_diagnostic()?)
        .await?;

    let req = Request::delete(format!("/{}", cmd.token_id));

    orchestrator_client.request(req).await?;

    println!("Revoked influxdb token {}.", cmd.token_id);

    Ok(())
}
