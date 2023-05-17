use clap::Args;

use ockam::Context;
use ockam_api::nodes::models::forwarder::ForwarderInfo;
use ockam_core::api::Request;

use crate::node::{default_node_name, get_node_name};
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;

/// Show a Relay by its alias
#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Name assigned to relay that will be shown (prefixed with forward_to_<name>)
    #[arg(display_order = 900, required = true)]
    remote_address: String,

    /// Node which relay belongs to
    #[arg(display_order = 901, global = true, long, value_name = "NODE")]
    pub at: Option<String>,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> Result<()> {
    let at = cmd
        .at
        .clone()
        .unwrap_or_else(|| default_node_name(&opts.state));
    let node_name = extract_address_value(&at)?;
    let remote_address = &cmd.remote_address;
    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    rpc.request(Request::get(format!("/node/forwarder/{remote_address}")))
        .await?;
    let relay_info_response = rpc.parse_response::<ForwarderInfo>()?;

    rpc.is_ok()?;

    println!("Relay:");
    println!("  Relay Route: {}", relay_info_response.forwarding_route());
    println!(
        "  Remote Address: {}",
        relay_info_response.remote_address_ma()?
    );
    println!(
        "  Worker Address: {}",
        relay_info_response.worker_address_ma()?
    );

    Ok(())
}
