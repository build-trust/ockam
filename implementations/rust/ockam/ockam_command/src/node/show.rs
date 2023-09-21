use clap::Args;
use colorful::Colorful;
use tokio_retry::strategy::FixedInterval;
use tracing::{info, trace, warn};

use ockam_api::cli_state::{CliState, StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::portal::{InletList, OutletList};
use ockam_api::nodes::models::secure_channel::SecureChannelListenersList;
use ockam_api::nodes::models::services::ServiceList;
use ockam_api::nodes::models::transport::TransportList;
use ockam_api::nodes::RemoteNode;
use ockam_api::{addr_to_multiaddr, route_to_multiaddr};
use ockam_core::Route;
use ockam_multiaddr::proto::{DnsAddr, Node, Tcp};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::node::get_node_name;
use crate::node::util::check_default;
use crate::util::{api, node_rpc};
use crate::{docs, CommandGlobalOpts, OutputFormat, Result};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

const IS_NODE_UP_TIME_BETWEEN_CHECKS_MS: usize = 50;
const IS_NODE_UP_MAX_ATTEMPTS: usize = 60; // 3 seconds

/// Show the details of a node
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name of the node to retrieve the details from
    #[arg()]
    node_name: Option<String>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_name);
    let mut node = RemoteNode::create(&ctx, &opts.state, &node_name).await?;
    let is_default = check_default(&opts, &node_name);
    print_query_status(&opts, &ctx, &mut node, false, is_default).await?;
    Ok(())
}

// TODO: This function should be replaced with a better system of
// printing the node state in the future but for now we can just tell
// clippy to stop complaining about it.
#[allow(clippy::too_many_arguments)]
fn print_node_info(
    opts: &CommandGlobalOpts,
    node_port: Option<u16>,
    node_name: &str,
    is_default: bool,
    status_is_up: bool,
    default_id: Option<&str>,
    services: Option<&ServiceList>,
    tcp_listeners: Option<&TransportList>,
    secure_channel_listeners: Option<&SecureChannelListenersList>,
    inlets_outlets: Option<(&InletList, &OutletList)>,
) {
    if opts.global_args.output_format == OutputFormat::Json {
        opts.terminal
            .clone()
            .stdout()
            .json(serde_json::json!({ "name": &node_name }))
            .write_line()
            .expect("Failed to write to stdout.");
        return;
    }
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
        for e in &list.list {
            println!("    Listener:");
            if let Some(ma) = addr_to_multiaddr(e.addr.clone()) {
                println!("      Address: {ma}");
                println!("      FlowControlId: {}", &e.flow_control_id);
            }
        }
    }

    if let Some((inlets, outlets)) = inlets_outlets {
        println!("  Inlets:");
        for e in &inlets.list {
            println!("    Inlet:");
            println!("      Listen Address: {}", e.bind_addr);
            if let Some(r) = Route::parse(&e.outlet_route) {
                if let Some(ma) = route_to_multiaddr(&r) {
                    println!("      Route To Outlet: {ma}");
                }
            }
        }
        println!("  Outlets:");
        for e in &outlets.list {
            println!("    Outlet:");
            println!("      Forward Address: {}", e.socket_addr);

            if let Some(ma) = addr_to_multiaddr(e.worker_addr.to_string()) {
                println!("      Address: {ma}");
            }
        }
    }

    if let Some(list) = services {
        println!("  Services:");
        for e in &list.list {
            println!("    Service:");
            println!("      Type: {}", e.service_type);
            if let Some(ma) = addr_to_multiaddr(e.addr.as_str()) {
                println!("      Address: {ma}");
            }
        }
    }
}

pub async fn print_query_status(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &mut RemoteNode,
    wait_until_ready: bool,
    is_default: bool,
) -> miette::Result<()> {
    let cli_state = opts.state.clone();
    if !is_node_up(ctx, node, cli_state.clone(), wait_until_ready).await? {
        let node_state = cli_state.nodes.get(node.node_name())?;
        let node_port = node_state
            .config()
            .setup()
            .api_transport()
            .ok()
            .map(|listener| listener.addr.port());

        // it is expected to not be able to open an arbitrary TCP connection on an authority node
        // so in that case we display an UP status
        let is_authority_node = node_state.config().setup().authority_node.unwrap_or(false);
        print_node_info(
            opts,
            node_port,
            node.node_name(),
            is_default,
            is_authority_node,
            None,
            None,
            None,
            None,
            None,
        );
    } else {
        let node_state = cli_state.nodes.get(node.node_name())?;
        // Get short id for the node
        let default_id = match node_state.config().identity_config() {
            Ok(resp) => resp.identifier().to_string(),
            Err(_) => String::from("None"),
        };

        // Get list of services for the node
        let services: ServiceList = node.ask(ctx, api::list_services()).await?;

        // Get list of TCP listeners for node
        let tcp_listeners: TransportList = node.ask(ctx, api::list_tcp_listeners()).await?;

        // Get list of Secure Channel Listeners
        let secure_channel_listeners: SecureChannelListenersList =
            node.ask(ctx, api::list_secure_channel_listener()).await?;

        // Get list of inlets
        let inlets: InletList = node.ask(ctx, api::list_inlets()).await?;

        // Get list of outlets
        let outlets: OutletList = node.ask(ctx, api::list_outlets()).await?;

        let node_state = cli_state.nodes.get(node.node_name())?;
        let node_port = node_state
            .config()
            .setup()
            .api_transport()
            .ok()
            .map(|listener| listener.addr.port());

        print_node_info(
            opts,
            node_port,
            node.node_name(),
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
pub async fn is_node_up(
    ctx: &Context,
    node: &mut RemoteNode,
    cli_state: CliState,
    wait_until_ready: bool,
) -> Result<bool> {
    let attempts = match wait_until_ready {
        true => IS_NODE_UP_MAX_ATTEMPTS,
        false => 1,
    };

    let retries =
        FixedInterval::from_millis(IS_NODE_UP_TIME_BETWEEN_CHECKS_MS as u64).take(attempts);

    let cli_state = cli_state.clone();
    let node_name = node.node_name().to_owned();
    let now = std::time::Instant::now();
    for timeout_duration in retries {
        let node_state = cli_state.nodes.get(&node_name)?;
        // The node is down if it has not stored its default tcp listener in its state file.
        if node_state.config().setup().api_transport().is_err() {
            trace!(%node_name, "node has not been initialized");
            tokio::time::sleep(timeout_duration).await;
            continue;
        }

        // Test if node is up
        // If node is down, we expect it won't reply and the timeout
        // will trigger the next loop (i.e. no need to sleep here).
        let result = node
            .set_timeout(timeout_duration)
            .tell(ctx, api::query_status())
            .await;
        if result.is_ok() {
            let elapsed = now.elapsed();
            info!(%node_name, ?elapsed, "node is up");
            return Ok(true);
        } else {
            trace!(%node_name, "node is initializing");
            tokio::time::sleep(timeout_duration).await;
        }
    }
    warn!(%node_name, "node didn't respond in time");
    Ok(false)
}
