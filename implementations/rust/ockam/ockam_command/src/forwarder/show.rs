use clap::Args;

use ockam::Context;
use ockam_api::nodes::models::forwarder::ForwarderInfo;
use ockam_core::api::Request;

use crate::node::{default_node_name, node_name_parser};
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;

/// Show a Forwarder by its alias
#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Name assigned to forwarder that will be shown (prefixed with forward_to_<name>)
    #[arg(display_order = 900, required = true)]
    remote_address: String,

    /// Node which forwarder belongs to
    #[arg(display_order = 901, global = true, long, value_name = "NODE", default_value_t = default_node_name(), value_parser = node_name_parser)]
    pub at: String,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> Result<()> {
    let node_name = extract_address_value(&cmd.at)?;
    let remote_address = &cmd.remote_address;
    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    rpc.request(Request::get(format!("/node/forwarder/{remote_address}")))
        .await?;
    let forwarder_info_response = rpc.parse_response::<ForwarderInfo>()?;

    rpc.is_ok()?;

    println!("Forwarder:");
    println!(
        "  Forwarding Route: {}",
        forwarder_info_response.forwarding_route()
    );
    println!(
        "  Remote Address: {}",
        forwarder_info_response.remote_address()
    );
    println!(
        "  Worker Address: {}",
        forwarder_info_response.worker_address()
    );

    Ok(())
}
