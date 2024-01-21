use clap::Args;
use console::Term;
use miette::IntoDiagnostic;
use ockam_api::nodes::models::secure_channel::ListSecureChannelListenerResponse;
use tokio_retry::strategy::FixedInterval;
use tracing::{info, trace, warn};

use ockam_api::nodes::models::base::NodeStatus;
use ockam_api::nodes::models::portal::{InletList, OutletList};
use ockam_api::nodes::models::services::ServiceList;
use ockam_api::nodes::models::transport::TransportList;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_node::Context;

use crate::node::list;
use crate::terminal::tui::ShowCommandTui;
use crate::terminal::PluralTerm;
use crate::util::{api, node_rpc};
use crate::{docs, CommandGlobalOpts, Result, Terminal, TerminalStream};

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
    ShowTui::run(ctx, opts, cmd.node_name).await
}

pub struct ShowTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node_name: Option<String>,
}

impl ShowTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        node_name: Option<String>,
    ) -> miette::Result<()> {
        let tui = Self {
            ctx,
            opts,
            node_name,
        };
        tui.show().await
    }
}

#[ockam_core::async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Node;

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.node_name.as_deref()
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        Ok(self
            .opts
            .state
            .get_node_or_default(&self.node_name)
            .await?
            .name())
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self
            .opts
            .state
            .get_nodes()
            .await?
            .iter()
            .map(|n| n.name())
            .collect())
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let mut node =
            BackgroundNodeClient::create(&self.ctx, &self.opts.state, &Some(item_name.to_string()))
                .await?;
        print_query_status(&self.opts, &self.ctx, &mut node, false).await?;
        Ok(())
    }

    async fn show_multiple(&self, items_names: Vec<String>) -> miette::Result<()> {
        let nodes = list::get_nodes_info(&self.opts, items_names).await?;
        list::print_nodes_info(&self.opts, nodes)?;
        Ok(())
    }
}

pub async fn print_query_status(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &mut BackgroundNodeClient,
    wait_until_ready: bool,
) -> miette::Result<()> {
    let cli_state = opts.state.clone();
    let node_name = node.node_name();
    let node_info = cli_state.get_node(&node_name).await?;

    let show_node = if !is_node_up(ctx, node, wait_until_ready).await? {
        // it is expected to not be able to open an arbitrary TCP connection on an authority node
        // so in that case we display an UP status
        let is_authority_node = cli_state
            .get_node(&node_name)
            .await
            .ok()
            .map(|n| n.is_authority_node())
            .unwrap_or(false);

        ShowNodeResponse::new(
            node_info.is_default(),
            &node_name,
            is_authority_node,
            node_info.tcp_listener_port(),
            node_info.pid(),
        )
    } else {
        let mut show_node = ShowNodeResponse::new(
            node_info.is_default(),
            &node_name,
            true,
            node_info.tcp_listener_port(),
            node_info.pid(),
        );
        // Get list of services for the node
        let services: ServiceList = node.ask(ctx, api::list_services()).await?;
        show_node.services = services
            .list
            .into_iter()
            .map(ShowServiceStatus::from)
            .collect();

        // Get list of TCP listeners for node
        let transports: TransportList = node.ask(ctx, api::list_tcp_listeners()).await?;
        show_node.transports = transports
            .list
            .into_iter()
            .map(ShowTransportStatus::from)
            .collect();

        // Get list of Secure Channel Listeners
        let listeners: ListSecureChannelListenerResponse =
            node.ask(ctx, api::list_secure_channel_listener()).await?;
        show_node.secure_channel_listeners = listeners
            .list
            .into_iter()
            .map(ShowSecureChannelListener::from)
            .collect();

        // Get list of inlets
        let inlets: InletList = node.ask(ctx, api::list_inlets()).await?;
        show_node.inlets = inlets.list.into_iter().map(ShowInletStatus::from).collect();

        // Get list of outlets
        let outlets: OutletList = node.ask(ctx, api::list_outlets()).await?;
        show_node.outlets = outlets
            .list
            .into_iter()
            .map(ShowOutletStatus::from)
            .collect();

        show_node
    };

    opts.terminal
        .clone()
        .stdout()
        .plain(&show_node)
        .json(serde_json::to_string_pretty(&show_node).into_diagnostic()?)
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
    node_client: &mut BackgroundNodeClient,
    wait_until_ready: bool,
) -> Result<bool> {
    let attempts = match wait_until_ready {
        true => IS_NODE_UP_MAX_ATTEMPTS,
        false => 1,
    };

    let retries =
        FixedInterval::from_millis(IS_NODE_UP_TIME_BETWEEN_CHECKS_MS as u64).take(attempts);

    let now = std::time::Instant::now();
    let node_name = node_client.node_name();

    for timeout_duration in retries {
        // The node is down if its default tcp listener has not been started yet
        let node = node_client.cli_state().get_node(&node_name).await.ok();
        let node_tcp_listener_address = node.as_ref().and_then(|n| n.tcp_listener_address());

        if node.is_none() || node_tcp_listener_address.is_none() {
            trace!(%node_name, "node has not been initialized");
            tokio::time::sleep(timeout_duration).await;
            continue;
        }

        // Test if node is up
        // If node is down, we expect it won't reply and the timeout
        // will trigger the next loop (i.e. no need to sleep here).
        let result = node_client
            .set_timeout(timeout_duration)
            .ask::<(), NodeStatus>(ctx, api::query_status())
            .await;
        if let Ok(node_status) = result {
            let elapsed = now.elapsed();
            info!(%node_name, ?elapsed, "node is up {:?}", node_status);
            return Ok(true);
        } else {
            trace!(%node_name, "node is initializing");
            tokio::time::sleep(timeout_duration).await;
        }
    }
    warn!(%node_name, "node didn't respond in time");
    Ok(false)
}
