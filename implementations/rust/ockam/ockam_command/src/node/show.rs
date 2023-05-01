use crate::node::util::check_default;
use crate::node::{default_node_name, node_name_parser};
use crate::util::{api, node_rpc, Rpc, RpcBuilder};
use crate::{docs, CommandGlobalOpts, Result};
use clap::Args;
use colorful::Colorful;
use ockam::TcpTransport;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::portal::{InletList, OutletList};
use ockam_api::nodes::models::services::ServiceList;
use ockam_api::nodes::models::transport::TransportList;
use ockam_api::{addr_to_multiaddr, cli_state, route_to_multiaddr};
use ockam_core::Route;
use ockam_multiaddr::proto::{DnsAddr, Node, Tcp};
use ockam_multiaddr::MultiAddr;
use tokio_retry::strategy::FixedInterval;
use tracing::{info, trace, warn};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

const IS_NODE_UP_TIME_BETWEEN_CHECKS_MS: usize = 50;
const IS_NODE_UP_MAX_ATTEMPTS: usize = 20; // 1 second

/// Show the details of a node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name of the node.
    #[arg(default_value_t = default_node_name(), value_parser = node_name_parser)]
    node_name: String,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> crate::Result<()> {
    let node_name = &cmd.node_name;

    let tcp = TcpTransport::create(&ctx).await?;
    let mut rpc = RpcBuilder::new(&ctx, &opts, node_name).tcp(&tcp)?.build();
    let is_default = check_default(&opts, node_name);
    print_query_status(&mut rpc, node_name, false, is_default).await?;
    Ok(())
}

// TODO: This function should be replaced with a better system of
// printing the node state in the future but for now we can just tell
// clippy to stop complaining about it.
#[allow(clippy::too_many_arguments)]
fn print_node_info(
    node_port: Option<u16>,
    node_name: &str,
    is_default: bool,
    status_is_up: bool,
    default_id: Option<&str>,
    services: Option<&ServiceList>,
    tcp_listeners: Option<&TransportList>,
    secure_channel_listeners: Option<&Vec<String>>,
    inlets_outlets: Option<(&InletList, &OutletList)>,
) {
    println!();
    println!("Node:");
    if is_default {
        println!("  Name: {node_name} (Default)");
    } else {
        println!("  Name: {node_name}");
    }
    println!(
        "  Status: {}",
        match status_is_up {
            true => "UP".light_green(),
            false => "DOWN".light_red(),
        }
    );

    println!("  Route To Node:");
    let mut m = MultiAddr::default();
    if m.push_back(Node::new(node_name)).is_ok() {
        println!("    Short: {m}");
    }

    if let Some(port) = node_port {
        let mut m = MultiAddr::default();
        if m.push_back(DnsAddr::new("localhost")).is_ok() && m.push_back(Tcp::new(port)).is_ok() {
            println!("    Verbose: {m}");
        }
    }

    if let Some(id) = default_id {
        println!("  Identity: {id}");
    }

    if let Some(list) = tcp_listeners {
        println!("  Transports:");
        for e in &list.list {
            println!("    Transport:");
            println!("      Type: {}", e.tt);
            println!("      Mode: {}", e.tm);
            println!("      Socket: {}", e.socket_addr);
            println!("      Worker: {}", e.worker_addr);
            println!("      FlowControlId: {}", e.flow_control_id);
        }
    }

    if let Some(list) = secure_channel_listeners {
        println!("  Secure Channel Listeners:");
        for e in list {
            println!("    Listener:");
            if let Some(ma) = addr_to_multiaddr(e) {
                println!("      Address: {ma}");
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
                    println!("      Route To Outlet: {ma}");
                }
            }
        }
        println!("  Outlets:");
        for e in &outlets.list {
            println!("    Outlet:");
            println!("      Forward Address: {}", e.tcp_addr);

            if let Some(ma) = addr_to_multiaddr(e.worker_addr.as_ref()) {
                println!("      Address: {ma}");
            }
        }
    }

    if let Some(list) = services {
        println!("  Services:");
        for e in &list.list {
            println!("    Service:");
            println!("      Type: {}", e.service_type);
            if let Some(ma) = addr_to_multiaddr(e.addr.as_ref()) {
                println!("      Address: {ma}");
            }
        }
    }
}

pub async fn print_query_status(
    rpc: &mut Rpc<'_>,
    node_name: &str,
    wait_until_ready: bool,
    is_default: bool,
) -> Result<()> {
    let cli_state = cli_state::CliState::try_default()?;
    if !is_node_up(rpc, wait_until_ready).await? {
        let node_state = cli_state.nodes.get(node_name)?;
        let node_port = node_state
            .config()
            .setup()
            .default_tcp_listener()
            .ok()
            .map(|listener| listener.addr.port());

        // it is expected to not be able to open an arbitrary TCP connection on an authority node
        // so in that case we display an UP status
        let is_authority_node = node_state.config().setup().authority_node.unwrap_or(false);
        print_node_info(
            node_port,
            node_name,
            is_default,
            is_authority_node,
            None,
            None,
            None,
            None,
            None,
        );
    } else {
        let node_state = cli_state.nodes.get(node_name)?;
        // Get short id for the node
        let default_id = match node_state.config().identity_config() {
            Ok(resp) => resp.identity.identifier().to_string(),
            Err(_) => String::from("None"),
        };

        // Get list of services for the node
        let mut rpc = rpc.clone();
        rpc.request(api::list_services()).await?;
        let services = rpc.parse_response::<ServiceList>()?;

        // Get list of TCP listeners for node
        let mut rpc = rpc.clone();
        rpc.request(api::list_tcp_listeners()).await?;
        let tcp_listeners = rpc.parse_response::<TransportList>()?;

        // Get list of Secure Channel Listeners
        let mut rpc = rpc.clone();
        rpc.request(api::list_secure_channel_listener()).await?;
        let secure_channel_listeners = rpc.parse_response::<Vec<String>>()?;

        // Get list of inlets
        let mut rpc = rpc.clone();
        rpc.request(api::list_inlets()).await?;
        let inlets = rpc.parse_response::<InletList>()?;

        // Get list of outlets
        let mut rpc = rpc.clone();
        rpc.request(api::list_outlets()).await?;
        let outlets = rpc.parse_response::<OutletList>()?;

        let node_state = cli_state.nodes.get(node_name)?;
        let node_port = node_state
            .config()
            .setup()
            .default_tcp_listener()
            .ok()
            .map(|listener| listener.addr.port());

        print_node_info(
            node_port,
            node_name,
            is_default,
            true,
            Some(&default_id),
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
pub async fn is_node_up(rpc: &mut Rpc<'_>, wait_until_ready: bool) -> Result<bool> {
    let attempts = match wait_until_ready {
        true => IS_NODE_UP_MAX_ATTEMPTS,
        false => 1,
    };

    let timeout =
        FixedInterval::from_millis(IS_NODE_UP_TIME_BETWEEN_CHECKS_MS as u64).take(attempts);

    let cli_state = cli_state::CliState::try_default()?;
    let node_name = rpc.node_name().to_owned();
    let now = std::time::Instant::now();
    for t in timeout {
        let node_state = cli_state.nodes.get(&node_name)?;
        // The node is down if it has not stored its default tcp listener in its state file.
        if node_state.config().setup().default_tcp_listener().is_err() {
            trace!(%node_name, "node has not been initialized");
            tokio::time::sleep(t).await;
            continue;
        }

        // Test if node is up
        // If node is down, we expect it won't reply and the timeout
        // will trigger the next loop (i.e. no need to sleep here).
        if rpc
            .request_with_timeout(api::query_status(), t)
            .await
            .is_ok()
            && rpc.is_ok().is_ok()
        {
            let elapsed = now.elapsed();
            info!(%node_name, ?elapsed, "node is up");
            return Ok(true);
        } else {
            trace!(%node_name, "node is initializing");
            tokio::time::sleep(t).await;
        }
    }
    warn!(%node_name, "node didn't respond in time");
    Ok(false)
}
