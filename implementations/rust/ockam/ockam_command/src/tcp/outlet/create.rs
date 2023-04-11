use crate::node::{default_node_name, node_name_parser};
use crate::tcp::util::alias_parser;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::{
    error::ApiError,
    nodes::models::portal::{CreateOutlet, OutletStatus},
    route_to_multiaddr,
};
use ockam_core::api::{Request, RequestBuilder};
use ockam_core::route;
use std::net::SocketAddr;

/// Create TCP Outlets
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Node on which to start the tcp outlet.
    #[arg(long, display_order = 900, id = "NODE", default_value_t = default_node_name(), value_parser = node_name_parser)]
    at: String,

    /// Address of the tcp outlet.
    #[arg(long, display_order = 901, id = "OUTLET_ADDRESS")]
    from: String,

    /// TCP address to send raw tcp traffic.
    #[arg(long, display_order = 902, id = "SOCKET_ADDRESS")]
    to: SocketAddr,

    /// Assign a name to this outlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

pub async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let node = extract_address_value(&cmd.at)?;
    let mut rpc = Rpc::background(&ctx, &options, &node)?;

    let cmd = CreateCommand {
        from: extract_address_value(&cmd.from)?,
        ..cmd
    };

    rpc.request(make_api_request(cmd)?).await?;
    let OutletStatus { worker_addr, .. } = rpc.parse_response()?;

    let addr = route_to_multiaddr(&route![worker_addr.to_string()])
        .ok_or_else(|| ApiError::generic("Invalid Outlet Address"))?;
    println!("{addr}");

    Ok(())
}

/// Construct a request to create a tcp outlet
fn make_api_request<'a>(cmd: CreateCommand) -> crate::Result<RequestBuilder<'a, CreateOutlet<'a>>> {
    let tcp_addr = cmd.to.to_string();
    let worker_addr = cmd.from;
    let alias = cmd.alias.map(|a| a.into());
    let payload = CreateOutlet::new(tcp_addr, worker_addr, alias);
    let request = Request::post("/node/outlet").body(payload);
    Ok(request)
}
