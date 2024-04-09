use std::ops::Add;
use std::time::Duration;

use clap::Args;
use console::Term;
use miette::IntoDiagnostic;
use tokio_retry::strategy::FibonacciBackoff;
use tracing::{info, trace, warn};

use ockam_api::nodes::models::base::NodeStatus;
use ockam_api::nodes::models::portal::{InletList, OutletList};
use ockam_api::nodes::models::secure_channel::ListSecureChannelListenerResponse;
use ockam_api::nodes::models::services::ServiceList;
use ockam_api::nodes::models::transport::TransportList;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::AsyncTryClone;
use ockam_node::Context;

use crate::terminal::tui::ShowCommandTui;
use crate::tui::PluralTerm;
use crate::util::{api, async_cmd};
use crate::{docs, CommandGlobalOpts, Result};

use super::models::portal::{ShowInletStatus, ShowOutletStatus};
use super::models::secure_channel::ShowSecureChannelListener;
use super::models::services::ShowServiceStatus;
use super::models::show::ShowNodeResponse;
use super::models::transport::ShowTransportStatus;

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

const IS_NODE_ACCESSIBLE_TIME_BETWEEN_CHECKS_MS: u64 = 100;
const IS_NODE_ACCESSIBLE_TIMEOUT: Duration = Duration::from_secs(180);

const IS_NODE_READY_TIME_BETWEEN_CHECKS_MS: u64 = 100;
const IS_NODE_READY_TIMEOUT: Duration = Duration::from_secs(180);

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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "node show".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ShowTui::run(ctx, opts, self.node_name.clone()).await
    }
}

pub struct ShowTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node_name: Option<String>,
}

impl ShowTui {
    pub async fn run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        node_name: Option<String>,
    ) -> miette::Result<()> {
        let tui = Self {
            ctx: ctx.async_try_clone().await.into_diagnostic()?,
            opts,
            node_name,
        };
        tui.show().await
    }
}

#[ockam_core::async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Node;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.node_name.clone()
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
    let node_name = node_client.node_name();
    if !is_node_accessible(ctx, node_client, wait_until_ready).await? {
        warn!(%node_name, "the node is not accessible in time");
        return Ok(false);
    }
    if !is_node_ready(ctx, node_client, wait_until_ready).await? {
        warn!(%node_name, "the node is not ready in time");
        return Ok(false);
    }
    Ok(true)
}

/// Return true if the node is accessible via TCP
async fn is_node_accessible(
    ctx: &Context,
    node_client: &mut BackgroundNodeClient,
    wait_until_ready: bool,
) -> Result<bool> {
    let retries = FibonacciBackoff::from_millis(IS_NODE_ACCESSIBLE_TIME_BETWEEN_CHECKS_MS);

    let node_name = node_client.node_name();

    let mut total_time = Duration::from_secs(0);
    for timeout_duration in retries {
        if total_time >= IS_NODE_ACCESSIBLE_TIMEOUT || !wait_until_ready && !total_time.is_zero() {
            return Ok(false);
        };
        if node_client.is_accessible(ctx).await.is_ok() {
            trace!(%node_name, "node is accessible");
            return Ok(true);
        }
        trace!(%node_name, "node is not accessible");
        tokio::time::sleep(timeout_duration).await;
        total_time = total_time.add(timeout_duration)
    }
    Ok(false)
}

/// Return true if the node has been initialized and is ready to accept requests
async fn is_node_ready(
    ctx: &Context,
    node_client: &mut BackgroundNodeClient,
    wait_until_ready: bool,
) -> Result<bool> {
    let retries = FibonacciBackoff::from_millis(IS_NODE_READY_TIME_BETWEEN_CHECKS_MS);

    let node_name = node_client.node_name();
    let now = std::time::Instant::now();
    let mut total_time = Duration::from_secs(0);
    for timeout_duration in retries {
        if total_time >= IS_NODE_READY_TIMEOUT || !wait_until_ready && !total_time.is_zero() {
            return Ok(false);
        };
        // Test if node is ready
        // If the node is down, we expect it won't reply and the timeout will trigger the next loop
        let result = node_client
            .ask_with_timeout::<(), NodeStatus>(ctx, api::query_status(), timeout_duration)
            .await;
        if let Ok(node_status) = result {
            let elapsed = now.elapsed();
            info!(%node_name, ?elapsed, "node is ready {:?}", node_status);
            return Ok(true);
        } else {
            trace!(%node_name, "node is initializing");
        }
        total_time = total_time.add(timeout_duration)
    }
    Ok(false)
}
