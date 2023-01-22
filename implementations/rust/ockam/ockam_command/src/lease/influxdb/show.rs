use anyhow::Context as _;
use clap::Args;
use ockam::Context;
use ockam_api::cloud::{
    lease_manager::models::influxdb::{ShowTokenRequest, ShowTokenResponse},
    CloudRequestWrapper,
};
use ockam_core::api::Request;

use crate::{
    lease::LeaseArgs,
    node::util::delete_embedded_node,
    util::{node_rpc, Rpc},
    CommandGlobalOpts,
};

/// InfluxDB Token Manager Add On
#[derive(Clone, Debug, Args)]
pub struct InfluxDbShowCommand {
    /// ID of the token to retrieve
    #[arg(short, long, value_name = "INFLUX_DB_TOKEN_ID")]
    pub token_id: String,
}

impl InfluxDbShowCommand {
    pub fn run(self, opts: CommandGlobalOpts, lease_args: LeaseArgs) {
        node_rpc(run_impl, (opts, lease_args, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, lease_args, cmd): (CommandGlobalOpts, LeaseArgs, InfluxDbShowCommand),
) -> crate::Result<()> {
    // TODO: Update show with orchestrator client

    // let controller_route = &lease_args.cloud_opts.route();
    // let mut rpc = Rpc::embedded(&ctx, &opts).await?;

    // let base_endpoint = |project_name: &str| -> crate::Result<String> {
    //     let lookup = opts.config.lookup();
    //     let project_id = &lookup
    //         .get_project(project_name)
    //         .context(format!(
    //             "Failed to get project {} from config lookup",
    //             project_name
    //         ))?
    //         .id;
    //     Ok(format!("{project_id}/lease_manager"))
    // };

    // let body = ShowTokenRequest::new(cmd.token_id.clone());

    // // e.g. API Path: GET "<proj_id>/lease_manager/influxdb/tokens"
    // // TODO: @oakley ADD ON TYPE shouldn't be a magic string
    // let add_on_id = "influxdb";
    // let node_api_path = format!(
    //     "{}/{}/{}/{}",
    //     base_endpoint(&lease_args.project_name)?,
    //     add_on_id,
    //     "tokens",
    //     cmd.token_id
    // );

    // let req = Request::get(node_api_path).body(CloudRequestWrapper::new(body, controller_route));
    // rpc.request(req).await?;
    // rpc.is_ok()?;

    // let res: ShowTokenResponse = rpc.parse_response()?;
    // // TODO : Create View for showing token
    // println!("Retrieving Token Info");

    // delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
