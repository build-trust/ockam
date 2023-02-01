use std::str::FromStr;

use clap::Args;
use ockam::Context;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::{
    help,
    util::{
        api::{CloudOpts, ProjectOpts},
        node_rpc,
        orchestrator_api::OrchestratorApiBuilder,
    },
    CommandGlobalOpts,
};

const HELP_DETAIL: &str = "";

/// Revoke a token within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = help::template(HELP_DETAIL))]
pub struct RevokeCommand {
    /// ID of the token to revoke
    #[arg(long, short, id = "token_id", value_name = "TOKEN_ID")]
    pub token_id: String,
}

impl RevokeCommand {
    pub fn run(self, options: CommandGlobalOpts, cloud_opts: CloudOpts, project_opts: ProjectOpts) {
        node_rpc(run_impl, (options, cloud_opts, self, project_opts));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, cmd, prjoect_opts): (
        CommandGlobalOpts,
        CloudOpts,
        RevokeCommand,
        ProjectOpts,
    ),
) -> crate::Result<()> {
    let mut orchestrator_client = OrchestratorApiBuilder::new(&ctx, &opts, &prjoect_opts)
        .as_identity(cloud_opts.identity.clone())
        .with_new_embbeded_node()
        .await?
        .build(&MultiAddr::from_str("/service/influxdb_token_lease")?)
        .await?;

    let req = Request::delete(format!("/{}", cmd.token_id));

    orchestrator_client.request(req).await?;

    println!("Revoked influxdb token {}.", cmd.token_id);

    Ok(())
}
