use anyhow::anyhow;
use clap::Args;

use ockam::Context;
use ockam_api::nodes::models::forwarder::ForwarderInfo;
use ockam_core::api::Request;

use crate::node::default_node_name;
use crate::util::{exitcode, extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;

/// List Relays
#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    /// Node to list relays from
    #[arg(global = true, long, value_name = "NODE", default_value_t = default_node_name())]
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
            anyhow!("No relays found on node {node_name}"),
        ));
    }

    // TODO: Switch to the table?
    for relay_info in response.iter() {
        println!("Relay:");
        println!("  Relay Route: {}", relay_info.forwarding_route());
        println!("  Remote Address: {}", relay_info.remote_address_ma()?);
        println!("  Worker Address: {}", relay_info.worker_address_ma()?);
        println!(
            "  Flow Control Id: {}",
            relay_info
                .flow_control_id()
                .as_ref()
                .map(|x| x.to_string())
                .unwrap_or("<none>".into())
        );
    }

    Ok(())
}
