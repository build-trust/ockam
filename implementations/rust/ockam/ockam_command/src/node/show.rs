use clap::Args;
use miette::IntoDiagnostic;
use ockam_api::nodes::models::secure_channel::SecureChannelListenersList;
use ockam_api::nodes::models::services::ServiceList;
use ockam_api::nodes::models::transport::TransportList;
use ockam_api::nodes::BackgroundNode;
use ockam_node::Context;
use tokio_retry::strategy::FixedInterval;
use tracing::{info, trace, warn};

use ockam_api::cli_state::{CliState, StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::portal::{InletList, OutletList};

use crate::node::get_node_name;
use crate::node::util::check_default;
use crate::util::{api, node_rpc};
use crate::{docs, CommandGlobalOpts, Result};

use super::models::portal::{ShowInletStatus, ShowOutletStatus};
use super::models::secure_channel::ShowSecureChannelListener;
use super::models::services::ShowServiceStatus;
use super::models::show::ShowNodeResponse;
use super::models::transport::ShowTransportStatus;

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
    let mut node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
    let is_default = check_default(&opts, &node_name);
    print_query_status(&opts, &ctx, &node_name, &mut node, false, is_default).await?;
    Ok(())
}

pub async fn print_query_status(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node_name: &str,
    node: &mut BackgroundNode,
    wait_until_ready: bool,
    is_default: bool,
) -> miette::Result<()> {
    let cli_state = opts.state.clone();

    let node_info =
        if !is_node_up(ctx, node_name, node, cli_state.clone(), wait_until_ready).await? {
            let node_state = cli_state.nodes.get(node_name)?;
            let node_port = node_state
                .config()
                .setup()
                .api_transport()
                .ok()
                .map(|listener| listener.addr.port());

            // it is expected to not be able to open an arbitrary TCP connection on an authority node
            // so in that case we display an UP status
            let is_authority_node = node_state.config().setup().authority_node.unwrap_or(false);

            ShowNodeResponse::new(is_default, node_name, is_authority_node, node_port)
        } else {
            let node_state = cli_state.nodes.get(node_name)?;
            let node_port = node_state
                .config()
                .setup()
                .api_transport()
                .ok()
                .map(|listener| listener.addr.port());

            let mut node_info = ShowNodeResponse::new(is_default, node_name, true, node_port);

            // Get short id for the node
            node_info.identity = Some(match node_state.config().identity_config() {
                Ok(resp) => resp.identifier().to_string(),
                Err(_) => String::from("None"),
            });

            // Get list of services for the node
            let services: ServiceList = node.ask(ctx, api::list_services()).await?;
            node_info.services = services
                .list
                .into_iter()
                .map(ShowServiceStatus::from)
                .collect();

            // Get list of TCP listeners for node
            let transports: TransportList = node.ask(ctx, api::list_tcp_listeners()).await?;
            node_info.transports = transports
                .list
                .into_iter()
                .map(ShowTransportStatus::from)
                .collect();

            // Get list of Secure Channel Listeners
            let listeners: SecureChannelListenersList =
                node.ask(ctx, api::list_secure_channel_listener()).await?;
            node_info.secure_channel_listeners = listeners
                .list
                .into_iter()
                .map(ShowSecureChannelListener::from)
                .collect();

            // Get list of inlets
            let inlets: InletList = node.ask(ctx, api::list_inlets()).await?;
            node_info.inlets = inlets.list.into_iter().map(ShowInletStatus::from).collect();

            // Get list of outlets
            let outlets: OutletList = node.ask(ctx, api::list_outlets()).await?;
            node_info.outlets = outlets
                .list
                .into_iter()
                .map(ShowOutletStatus::from)
                .collect();

            node_info
        };

    opts.terminal
        .clone()
        .stdout()
        .plain(&node_info)
        .json(serde_json::to_string_pretty(&node_info).into_diagnostic()?)
        .write_line()?;

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
    node_name: &str,
    node: &mut BackgroundNode,
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
    let now = std::time::Instant::now();
    for timeout_duration in retries {
        let node_state = cli_state.nodes.get(node_name)?;
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
