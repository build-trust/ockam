use crate::node::NodeOpts;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam_api::nodes::models;
use ockam_core::api::Request;
use ockam_core::Route;
use ockam_api::{route_to_multiaddr};

/// Retrieve inlets information on Node
#[derive(Args, Clone, Debug)]
pub struct ListCommand {
    #[command(flatten)]
    node: NodeOpts,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl,(options, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (options, command): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    let node_name = extract_address_value(&command.node.api_node)?;
    let mut rpc = Rpc::background(&ctx, &options, &node_name)?;
    rpc.request(Request::get("/node/inlet")).await?;
    let response = rpc.parse_response::<models::portal::InletList>()?;

    let mut inlet_infor = response.list.iter();
    loop {
        match inlet_infor.next() {
            Some(info) => {
                println!("Inlet:");
                println!("  Inlet Alias: {}", info.alias);
                println!("  TCP Address: {}", info.bind_addr);
                if let Some(r) = Route::parse(info.outlet_route.as_ref()) {
                    if let Some(ma) = route_to_multiaddr(&r) {
                        println!("  To Outlet Address: {ma}");
                    }
                }
                
            },
            None => break
        }
    }
    Ok(())
}
