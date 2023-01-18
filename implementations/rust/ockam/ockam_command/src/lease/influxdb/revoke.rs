use clap::Args;
use ockam::Context;
use ockam_api::cloud::{lease_manager::models::influxdb::RevokeTokenRequest, CloudRequestWrapper};
use ockam_core::api::Request;

use crate::{
    lease::LeaseArgs,
    node::util::delete_embedded_node,
    util::{node_rpc, Rpc},
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

    let body = RevokeTokenRequest::new(cmd.token_id.clone());

    let add_on_id = "influxdb";
    let node_api_path = format!(
        "{}/{}/{}/{}",
        base_endpoint(&lease_args.project_name)?,
        add_on_id,
        "tokens",
        cmd.token_id
    );

    let req = Request::delete(node_api_path).body(CloudRequestWrapper::new(body, controller_route));
    rpc.request(req).await?;
    rpc.is_ok()?;

    println!("Revoked influxdb token {}.", cmd.token_id);

    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
