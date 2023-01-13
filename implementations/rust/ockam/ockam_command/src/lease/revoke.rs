use std::{path::PathBuf, str::FromStr};

use clap::Args;
use ockam::Context;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::{
    help,
    util::{node_rpc, orchestrator_api::OrchestratorApiBuilder},
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
    pub fn run(self, options: CommandGlobalOpts, identity: Option<String>, project_path: PathBuf) {
        node_rpc(run_impl, (options, identity, project_path, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, identity, project_path, cmd): (
        CommandGlobalOpts,
        Option<String>,
        PathBuf,
        RevokeCommand,
    ),
) -> crate::Result<()> {
    let mut orchestrator_client = OrchestratorApiBuilder::new(&ctx, &opts)
        .as_identity(identity)
        .with_new_embbeded_node()
        .await?
        .with_project_from_file(&project_path)
        .await?
        .build(&MultiAddr::from_str("/service/influxdb_token_lease")?)
        .await?;

    let req = Request::delete(format!("/{}", cmd.token_id));

    orchestrator_client.request(req).await?;

    println!("Revoked influxdb token {}.", cmd.token_id);

    Ok(())
}
