use std::str::FromStr;

use anyhow::Context as _;
use clap::Args;
use ockam::Context;
use ockam_api::cloud::{
    lease_manager::models::influxdb::{ListTokensRequest, ListTokensResponse},
    CloudRequestWrapper,
};
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::{
    lease::LeaseArgs,
    node::util::delete_embedded_node,
    util::{node_rpc, orchestrator_api::OrchestratorApiBuilder, Rpc},
    CommandGlobalOpts,
};

/// InfluxDB Token Manager Add On
#[derive(Clone, Debug, Args)]
pub struct InfluxDbListCommand {
    /// Only show authorizations that belong to the provided user name.
    #[arg(long, group = "user_group", value_name = "USERNAME")]
    pub user: Option<String>,

    /// Only show authorizations that belong to the provided user ID.
    #[arg(long, group = "user_group", value_name = "USER_ID")]
    pub user_id: Option<String>,
}

impl InfluxDbListCommand {
    pub fn run(self, opts: CommandGlobalOpts, lease_args: LeaseArgs) {
        node_rpc(run_impl, (opts, lease_args, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, lease_args, cmd): (CommandGlobalOpts, LeaseArgs, InfluxDbListCommand),
) -> crate::Result<()> {
    let mut orchestrator_client = OrchestratorApiBuilder::new(&ctx, &opts)
        .as_identity(lease_args.cloud_opts.identity)
        .with_new_embbeded_node()
        .await?
        .with_project_from_file(&lease_args.project)
        .await?
        .build(&MultiAddr::from_str("/service")?)
        .await?;
    let body = ListTokensRequest::new(cmd.user, cmd.user_id);

    let req = Request::post("/lease_manager/influxdb/tokens").body(body);

    let resp: ListTokensResponse = orchestrator_client.request(req).await?;

    // TODO: Create view for listing tokens
    println!("List tokens.");
    Ok(())
}
