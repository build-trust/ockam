use clap::Args;
use ockam::Context;
use ockam_api::cloud::{lease_manager::models::influxdb::CreateTokenRequest, CloudRequestWrapper};
use ockam_core::api::Request;

use crate::{
    lease::LeaseArgs,
    node::util::delete_embedded_node,
    util::{node_rpc, Rpc},
    CommandGlobalOpts,
};
use anyhow::Context as _;

use super::InfluxDbTokenStatus;

/// InfluxDB Token Manager Add On
#[derive(Clone, Debug, Args)]
pub struct InfluxDbCreateCommand {
    /// Optional description of the token
    #[arg(long, id = "description", value_name = "TOKEN_DESCRIPTION")]
    pub description: Option<String>,

    /// Explicitly sets the status of the token
    /// If the token is inactive and requests using the token will be rejected.
    /// Defaults to Active
    #[arg(long, id = "status", value_name = "INFLUXDB_TOKEN_STATUS")]
    pub status: Option<InfluxDbTokenStatus>,

    /// ID of user the authorization is scoped to
    #[arg(long, id = "user_id", value_name = "CLIENT_ID")]
    pub user_id: Option<String>,
}

impl InfluxDbCreateCommand {
    pub fn run(self, opts: CommandGlobalOpts, lease_args: LeaseArgs) {
        node_rpc(run_impl, (opts, lease_args, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, lease_args, cmd): (CommandGlobalOpts, LeaseArgs, InfluxDbCreateCommand),
) -> crate::Result<()> {
    let controller_route = &lease_args.cloud_opts.route();
    let mut rpc = Rpc::embedded(&ctx, &opts).await?;

    let base_endpoint = |project_name: &str| -> crate::Result<String> {
        let lookup = opts.config.lookup();
        let project_id = &lookup
            .get_project(project_name)
            .context(format!(
                "Failed to get project {} from config lookup",
                project_name
            ))?
            .id;
        Ok(format!("{project_id}/lease_manager"))
    };

    let body = CreateTokenRequest::new(
        cmd.description,
        cmd.status.map(|s| s.to_string()),
        cmd.user_id,
    );

    // e.g. API Path: POST "<proj_id>/lease_manager/influxdb/tokens"
    let add_on_id = "influxdb";
    let node_api_path = format!(
        "{}/{}/{}",
        base_endpoint(&lease_args.project_name)?,
        add_on_id,
        "tokens"
    );

    let req = Request::post(node_api_path).body(CloudRequestWrapper::new(body, controller_route));
    rpc.request(req).await?;
    rpc.is_ok()?;

    println!("Created token within InfluxDB");

    // TODO: decode response and show token info

    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
