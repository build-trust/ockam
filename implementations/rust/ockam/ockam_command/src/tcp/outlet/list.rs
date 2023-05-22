use crate::node::{get_node_name, initialize_node, NodeOpts};
use crate::util::{exitcode, extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use anyhow::anyhow;
use clap::Args;
use ockam_api::nodes::models::portal::OutletList;
use ockam_api::{error::ApiError, route_to_multiaddr};
use ockam_core::api::Request;
use ockam_core::route;

/// List TCP Outlets
#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node(&opts, &self.node_opts.api_node);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.api_node);
    let node_name = extract_address_value(&node_name)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    rpc.request(Request::get("/node/outlet")).await?;
    let response = rpc.parse_response::<OutletList>()?;

    if response.list.is_empty() {
        return Err(crate::Error::new(
            exitcode::IOERR,
            anyhow!("No Outlets found on this system!"),
        ));
    }

    for outlet in &response.list {
        println!("Outlet:");
        println!("  Alias: {}", outlet.alias);
        let addr = route_to_multiaddr(&route![outlet.worker_addr.to_string()])
            .ok_or_else(|| ApiError::generic("Invalid Outlet Address"))?;
        println!("  From Outlet: {addr}");
        println!("  To TCP: {}", outlet.tcp_addr);
    }
    Ok(())
}
