use anyhow::Context as _;
use clap::Args;
use ockam::Context;
use ockam_api::cloud::{lease_manager::models::influxdb::ListTokensRequest, CloudRequestWrapper};
use ockam_core::api::Request;

use crate::{
    lease::LeaseArgs,
    node::util::delete_embedded_node,
    util::{node_rpc, Rpc},
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

    let body = ListTokensRequest::new(cmd.user, cmd.user_id);

    // e.g. API Path: GET "<proj_id>/lease_manager/influxdb/tokens"
    let add_on_id = "influxdb";
    let node_api_path = format!(
        "{}/{}/{}",
        base_endpoint(&lease_args.project_name)?,
        add_on_id,
        "tokens"
    );

    let req = Request::get(node_api_path).body(CloudRequestWrapper::new(body, controller_route));
    rpc.request(req).await?;
    rpc.is_ok()?;

    println!("Listing tokens within InfluxDB");

    // TODO: @oakley decode response and list tokens

    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
