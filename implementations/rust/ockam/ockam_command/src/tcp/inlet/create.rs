use crate::node::{default_node_name, node_name_parser};
use crate::policy::{add_default_project_policy, has_policy};
use crate::tcp::util::alias_parser;
use crate::util::{
    bind_to_port_check, exitcode, extract_address_value, find_available_port, node_rpc,
    process_nodes_multiaddr, RpcBuilder,
};
use crate::{CommandGlobalOpts, Result};

use anyhow::anyhow;
use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam::{Context, TcpTransport};
use ockam_abac::Resource;
use ockam_api::nodes::models::portal::CreateInlet;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_core::api::Request;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol as _};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

/// Create TCP Inlets
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Node on which to start the tcp inlet.
    #[arg(long, display_order = 900, id = "NODE", default_value_t = default_node_name(), value_parser = node_name_parser)]
    at: String,

    /// Address on which to accept tcp connections.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", default_value_t = default_from_addr())]
    from: SocketAddr,

    /// Route to a tcp outlet.
    #[arg(long, display_order = 900, id = "ROUTE", default_value_t = default_to_addr())]
    to: MultiAddr,

    /// Authorized identity for secure channel connection
    #[arg(long, name = "AUTHORIZED", display_order = 900)]
    authorized: Option<IdentityIdentifier>,

    /// Assign a name to this inlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,
}

fn default_from_addr() -> SocketAddr {
    let port = find_available_port().expect("Failed to find available port");
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
}

fn default_to_addr() -> MultiAddr {
    MultiAddr::from_str("/project/default/service/forward_to_default/secure/api/service/outlet")
        .expect("Failed to parse default multiaddr")
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, mut cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    cmd.to = process_nodes_multiaddr(&cmd.to, &opts.state)?;

    // Check if the port is used by some other services or process
    if !bind_to_port_check(&cmd.from) {
        return Err(crate::error::Error::new(
            exitcode::IOERR,
            anyhow!("Another process is listening on the provided port!"),
        ));
    }

    let tcp = TcpTransport::create(&ctx).await?;
    let node = extract_address_value(&cmd.at)?;
    let project = opts.state.nodes.get(&node)?.setup()?.project;
    let resource = Resource::new("tcp-inlet");
    if let Some(p) = project {
        if !has_policy(&node, &ctx, &opts, &resource).await? {
            add_default_project_policy(&node, &ctx, &opts, p, &resource).await?;
        }
    }

    let req = {
        let mut payload = if cmd.to.matches(0, &[Project::CODE.into()]) {
            if cmd.authorized.is_some() {
                return Err(anyhow!("--authorized can not be used with project addresses").into());
            }
            CreateInlet::via_project(cmd.from, cmd.to)
        } else {
            CreateInlet::to_node(cmd.from, cmd.to, cmd.authorized)
        };
        if let Some(a) = cmd.alias {
            payload.set_alias(a)
        }
        Request::post("/node/inlet").body(payload)
    };

    let mut rpc = RpcBuilder::new(&ctx, &opts, &node).tcp(&tcp)?.build();
    rpc.request(req).await?;
    let inlet = rpc.parse_response::<InletStatus>()?;

    let output = format!(
        r#"
    Inlet
        ID: {}
        Address: {}
        Worker: {}
        Outlet: {}
    "#,
        inlet.alias, inlet.bind_addr, inlet.worker_addr, inlet.outlet_route
    );

    let machine_output = inlet.bind_addr.to_string();

    let json_output = serde_json::to_string_pretty(&inlet)?;

    opts.shell
        .stdout()
        .plain(output)
        .machine(machine_output)
        .json(json_output)
        .write_line()?;

    Ok(())
}
