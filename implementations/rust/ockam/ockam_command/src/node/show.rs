use crate::util::{api, connect_to, exitcode, OckamConfig};
use crate::{help, node::HELP_DETAIL, CommandGlobalOpts};
use anyhow::Context;
use clap::Args;
use colorful::Colorful;
use minicbor::Decoder;
use ockam_api::config::cli::NodeConfigOld;
use ockam_api::nodes::models::portal::{InletList, OutletList};
use ockam_api::nodes::models::services::ServiceList;
use ockam_api::nodes::models::transport::TransportList;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_api::{addr_to_multiaddr, route_to_multiaddr};
use ockam_core::api::{Response, Status};
use ockam_core::{Result, Route};
use ockam_multiaddr::proto::{DnsAddr, Node, Tcp};
use ockam_multiaddr::MultiAddr;
use std::time::Duration;

const IS_NODE_UP_ATTEMPTS: usize = 10;
const IS_NODE_UP_SLEEP_MILLIS: u64 = 250;
const SEND_RECEIVE_TIMEOUT_SECS: u64 = 1;

/// Show Nodes
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct ShowCommand {
    /// Name of the node.
    #[arg(default_value = "default")]
    node_name: String,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = &options.config;
        let port = match cfg.inner().nodes.get(&self.node_name) {
            Some(cfg) => cfg.port(),
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };
        connect_to(
            port,
            (cfg.clone(), self.node_name, false),
            print_query_status,
        );
    }
}

// TODO: This function should be replaced with a better system of
// printing the node state in the future but for now we can just tell
// clippy to stop complainaing about it.
#[allow(clippy::too_many_arguments)]
fn print_node_info(
    node_cfg: &NodeConfigOld,
    node_name: &str,
    status: &str,
    default_id: &str,
    services: Option<&ServiceList>,
    tcp_listeners: Option<&TransportList>,
    secure_channel_listeners: Option<&Vec<String>>,
    inlets_outlets: Option<(&InletList, &OutletList)>,
) {
    println!();
    println!("Node:");
    println!("  Name: {}", node_name);
    println!(
        "  Status: {}",
        match status {
            "UP" => status.light_green(),
            "DOWN" => status.light_red(),
            _ => status.white(),
        }
    );

    println!("  Route To Node:");
    let mut m = MultiAddr::default();
    if m.push_back(Node::new(node_name)).is_ok() {
        println!("    Short: {}", m);
    }

    let mut m = MultiAddr::default();
    if m.push_back(DnsAddr::new("localhost")).is_ok()
        && m.push_back(Tcp::new(node_cfg.port())).is_ok()
    {
        println!("    Verbose: {}", m);
    }
    println!("  Identity: {}", default_id);

    if let Some(list) = tcp_listeners {
        println!("  Transports:");
        for e in &list.list {
            println!("    Transport:");
            println!("      Type: {}", e.tt);
            println!("      Mode: {}", e.tm);
            println!("      Address: {}", e.payload);
        }
    }

    if let Some(list) = secure_channel_listeners {
        println!("  Secure Channel Listeners:");
        for e in list {
            println!("    Listener:");
            if let Some(ma) = addr_to_multiaddr(e) {
                println!("      Address: {}", ma);
            }
        }
    }

    if let Some((inlets, outlets)) = inlets_outlets {
        println!("  Inlets:");
        for e in &inlets.list {
            println!("    Inlet:");
            println!("      Listen Address: {}", e.bind_addr);
            if let Some(r) = Route::parse(e.outlet_route.as_ref()) {
                if let Some(ma) = route_to_multiaddr(&r) {
                    println!("      Route To Outlet: {}", ma);
                }
            }
        }
        println!("  Outlets:");
        for e in &outlets.list {
            println!("    Outlet:");
            println!("      Forward Address: {}", e.tcp_addr);

            if let Some(ma) = addr_to_multiaddr(e.worker_addr.as_ref()) {
                println!("      Address: {}", ma);
            }
        }
    }

    if let Some(list) = services {
        println!("  Services:");
        for e in &list.list {
            println!("    Service:");
            println!("      Type: {}", e.service_type);
            if let Some(ma) = addr_to_multiaddr(e.addr.as_ref()) {
                println!("      Address: {}", ma);
            }
        }
    }
}

pub async fn print_query_status(
    mut ctx: ockam::Context,
    (cfg, node_name, wait_until_ready): (OckamConfig, String, bool),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let route = base_route.modify().append(NODEMANAGER_ADDR).into();
    let node_cfg = cfg.get_node(&node_name)?;

    if !is_node_up(&mut ctx, &route, wait_until_ready).await? {
        print_node_info(&node_cfg, &node_name, "DOWN", "N/A", None, None, None, None);
    } else {
        // Get short id for the node
        let resp: Vec<u8> = ctx
            .send_and_receive_with_timeout(
                route.clone(),
                api::short_identity().to_vec()?,
                SEND_RECEIVE_TIMEOUT_SECS,
            )
            .await
            .context("Failed to get short identity from node")?;
        let (response, result) = api::parse_short_identity_response(&resp)?;
        let default_id = match response.status() {
            Some(Status::Ok) => {
                format!("{}", result.identity_id)
            }
            _ => String::from("NOT FOUND"),
        };

        // Get list of services for the node
        let resp: Vec<u8> = ctx
            .send_and_receive_with_timeout(
                route.clone(),
                api::list_services().to_vec()?,
                SEND_RECEIVE_TIMEOUT_SECS,
            )
            .await
            .context("Failed to get list of services from node")?;
        let services = api::parse_list_services_response(&resp)?;

        // Get list of TCP listeners for node
        let resp: Vec<u8> = ctx
            .send_and_receive_with_timeout(
                route.clone(),
                api::list_tcp_listeners().to_vec()?,
                SEND_RECEIVE_TIMEOUT_SECS,
            )
            .await
            .context("Failed to get list of tcp listeners from node")?;
        let tcp_listeners = api::parse_tcp_list(&resp)?;

        // Get list of Secure Channel Listeners
        let resp: Vec<u8> = ctx
            .send_and_receive_with_timeout(
                route.clone(),
                api::list_secure_channel_listener().to_vec()?,
                SEND_RECEIVE_TIMEOUT_SECS,
            )
            .await
            .context("Failed to get list of secure channel listeners from node")?;
        let mut dec = Decoder::new(&resp);
        let _ = dec.decode::<Response>()?;
        let secure_channel_listeners = dec.decode::<Vec<String>>()?;

        // Get list of inlets
        let resp: Vec<u8> = ctx
            .send_and_receive_with_timeout(
                route.clone(),
                api::list_inlets().to_vec()?,
                SEND_RECEIVE_TIMEOUT_SECS,
            )
            .await
            .context("Failed to get list of inlets from node")?;
        let inlets = api::parse_list_inlets_response(&resp)?;

        // Get list of outlets
        let resp: Vec<u8> = ctx
            .send_and_receive_with_timeout(
                route.clone(),
                api::list_outlets().to_vec()?,
                SEND_RECEIVE_TIMEOUT_SECS,
            )
            .await
            .context("Failed to get list of outlets from node")?;
        let outlets = api::parse_list_outlets_response(&resp)?;

        print_node_info(
            &node_cfg,
            &node_name,
            "UP",
            &default_id,
            Some(&services),
            Some(&tcp_listeners),
            Some(&secure_channel_listeners),
            Some((&inlets, &outlets)),
        );
    }

    Ok(())
}

/// Send message(s) to a node to determine if it is 'up' and
/// responding to requests.
///
/// If `wait_until_ready` is `true` and the node does not
/// appear to be 'up', retry the test at time intervals up to
/// a maximum number of retries. A use case for this is to
/// allow a node time to start up and become ready.
async fn is_node_up(
    ctx: &mut ockam::Context,
    route: &Route,
    wait_until_ready: bool,
) -> anyhow::Result<bool> {
    let attempts = match wait_until_ready {
        true => IS_NODE_UP_ATTEMPTS,
        false => 1,
    };

    for att in 0..attempts {
        // Sleep, if this not the first loop
        if att > 0 {
            tokio::time::sleep(Duration::from_millis(IS_NODE_UP_SLEEP_MILLIS)).await;
        }

        // Test if node is up
        let tx_result: Result<Vec<u8>> = ctx
            .send_and_receive_with_timeout(
                route.clone(),
                api::query_status()?,
                SEND_RECEIVE_TIMEOUT_SECS,
            )
            .await;
        if let Ok(data) = tx_result {
            if api::parse_status(&data).is_ok() {
                // Node is up, return
                return Ok(true);
            }
        }
    }

    Ok(false)
}
