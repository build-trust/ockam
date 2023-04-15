use crate::node::{default_node_name, node_name_parser};
use crate::policy::{add_default_project_policy, has_policy};
use crate::tcp::util::alias_parser;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;
use anyhow::anyhow;
use clap::Args;
use ockam::Context;
use ockam_abac::Resource;
use ockam_api::{
    error::ApiError,
    nodes::models::portal::{CreateOutlet, OutletStatus},
    route_to_multiaddr,
};
use ockam_core::api::{Request, RequestBuilder};
use ockam_core::route;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Create TCP Outlets
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Node on which to start the tcp outlet.
    #[arg(long, display_order = 900, id = "NODE", default_value_t = default_node_name(), value_parser = node_name_parser)]
    at: String,

    /// Address of the tcp outlet.
    #[arg(long, display_order = 901, id = "OUTLET_ADDRESS", default_value_t = default_from_addr())]
    from: String,

    /// TCP address to send raw tcp traffic.
    #[arg(long, display_order = 902, id = "SOCKET_ADDRESS", value_parser = socket_addr_parser)]
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

fn socket_addr_parser(input: &str) -> Result<SocketAddr> {
    let to_address_info: Vec<&str> = input.split(':').collect();
    if to_address_info.len() > 2 {
        return Err(anyhow!("Failed to parse to address").into());
    }

    let port: u16 = if to_address_info.len() == 2 {
        to_address_info[1]
            .parse()
            .map_err(|_| anyhow!("Invalid port number"))?
    } else {
        to_address_info[0]
            .parse()
            .map_err(|_| anyhow!("Invalid port number"))?
    };

    let server_ip: Ipv4Addr = if to_address_info.len() < 2 {
        [127, 0, 0, 1].into()
    } else {
        let address_octets: [u8; 4] = {
            let mut octets = [0; 4];
            for (i, octet_str) in to_address_info[0].split('.').enumerate() {
                octets[i] = octet_str
                    .parse()
                    .map_err(|_| anyhow!("Invalid IP address"))?;
            }
            octets
        };
        Ipv4Addr::from(address_octets)
    };

    Ok(SocketAddr::new(IpAddr::V4(server_ip), port))
}

fn default_from_addr() -> String {
    "/service/outlet".to_string()
}

pub async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let node = extract_address_value(&cmd.at)?;
    let project = options.state.nodes.get(&node)?.setup()?.project;
    let resource = Resource::new("tcp-outlet");
    if let Some(p) = project {
        if !has_policy(&node, &ctx, &options, &resource).await? {
            add_default_project_policy(&node, &ctx, &options, p, &resource).await?;
        }
    }

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
