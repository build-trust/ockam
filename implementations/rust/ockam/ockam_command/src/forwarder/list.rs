use anyhow::anyhow;
use clap::Args;

use ockam::Context;
use ockam_api::nodes::models::forwarder::ForwarderInfo;
use ockam_core::api::Request;

use crate::node::{default_node_name, node_name_parser};
use crate::util::{exitcode, extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;

/// List Forwarders
#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    /// Node to list forwarders from
    #[arg(global = true, long, value_name = "NODE", default_value_t = default_node_name(), value_parser = node_name_parser)]
    pub at: String,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> Result<()> {
    let node_name = extract_address_value(&cmd.at)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    rpc.request(Request::get("/node/forwarder")).await?;
    let response = rpc.parse_response::<Vec<ForwarderInfo>>()?;

    if response.is_empty() {
        return Err(crate::Error::new(
            exitcode::IOERR,
            anyhow!("No Forwarders found on node {node_name}"),
        ));
    }

    for forwarder_info in response.iter() {
        println!("Forwarder:");
        println!("  Forwarding Route: {}", forwarder_info.forwarding_route());
        println!("  Remote Address: {}", forwarder_info.remote_address());
        println!("  Worker Address: {}", forwarder_info.worker_address());
    }

    Ok(())
}
