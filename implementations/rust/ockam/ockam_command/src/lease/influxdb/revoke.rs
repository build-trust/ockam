use std::str::FromStr;

use clap::Args;
use ockam::Context;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::{
    lease::LeaseArgs,
    node::util::delete_embedded_node,
    util::{node_rpc, orchestrator_api::OrchestratorApiBuilder, Rpc},
    CommandGlobalOpts,
};
use anyhow::Context as _;

/// InfluxDB Token Manager Add On
#[derive(Clone, Debug, Args)]
pub struct InfluxDbRevokeCommand {
    /// ID of the token to revoke
    #[arg(long, short, id = "token_id", value_name = "INFLUX_DB_TOKEN_ID")]
    pub token_id: String,
}

impl InfluxDbRevokeCommand {
    pub fn run(self, opts: CommandGlobalOpts, lease_args: LeaseArgs) {
        node_rpc(run_impl, (opts, lease_args, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, lease_args, cmd): (CommandGlobalOpts, LeaseArgs, InfluxDbRevokeCommand),
) -> crate::Result<()> {
    let mut orchestrator_client = OrchestratorApiBuilder::new(&ctx, &opts)
        .as_identity(lease_args.cloud_opts.identity)
        .with_new_embbeded_node()
        .await?
        .with_project_from_file(&lease_args.project)
        .await?
        .build(&MultiAddr::from_str("/service/influxdb_token_lease")?)
        .await?;

    let req =
        Request::delete(format!("/{}", cmd.token_id));

    // TOOD: add or change structure of client to allow for requests w/o responses.
    orchestrator_client.request(req).await?;

    println!("Revoked influxdb token {}.", cmd.token_id);

    Ok(())
}
