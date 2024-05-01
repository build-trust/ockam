use async_trait::async_trait;
use std::ops::Add;
use std::time::Duration;

use clap::Args;
use console::Term;
use miette::IntoDiagnostic;

use ockam_api::CliState;
use tokio_retry::strategy::FibonacciBackoff;
use tracing::{info, trace, warn};

use ockam_api::nodes::models::node::{NodeResources, NodeStatus};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::AsyncTryClone;
use ockam_node::Context;

use crate::terminal::tui::ShowCommandTui;
use crate::tui::PluralTerm;
use crate::util::api;
use crate::{docs, Command, CommandGlobalOpts, Result};

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

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "node show";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        Ok(ShowTui::run(ctx, opts, self.node_name.clone()).await?)
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
        let node_resources =
            get_node_resources(&self.ctx, &self.opts.state, &mut node, false).await?;
        self.opts
            .terminal
            .clone()
            .stdout()
            .plain(&node_resources)
            .json(serde_json::to_string(&node_resources).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}

pub async fn get_node_resources(
    ctx: &Context,
    cli_state: &CliState,
    node: &mut BackgroundNodeClient,
    wait_until_ready: bool,
) -> miette::Result<NodeResources> {
    let node_name = node.node_name();
    if is_node_up(ctx, node, wait_until_ready).await? {
        Ok(node.ask(ctx, api::get_node_resources()).await?)
    } else {
        let node_info = cli_state.get_node(&node_name).await?;
        let identity = cli_state
            .get_named_identity_by_identifier(&node_info.identifier())
            .await?;
        NodeResources::empty(node_info, identity.name()).into_diagnostic()
    }
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
    node: &mut BackgroundNodeClient,
    wait_until_ready: bool,
) -> Result<bool> {
    let node_name = node.node_name();
    if !is_node_accessible(ctx, node, wait_until_ready).await? {
        warn!(%node_name, "the node was not accessible in time");
        return Ok(false);
    }
    if !is_node_ready(ctx, node, wait_until_ready).await? {
        warn!(%node_name, "the node was not ready in time");
        return Ok(false);
    }
    Ok(true)
}

/// Return true if the node is accessible via TCP
async fn is_node_accessible(
    ctx: &Context,
    node: &mut BackgroundNodeClient,
    wait_until_ready: bool,
) -> Result<bool> {
    let node_name = node.node_name();
    let retries = FibonacciBackoff::from_millis(IS_NODE_ACCESSIBLE_TIME_BETWEEN_CHECKS_MS);
    let mut total_time = Duration::from_secs(0);
    for timeout_duration in retries {
        if total_time >= IS_NODE_ACCESSIBLE_TIMEOUT || !wait_until_ready && !total_time.is_zero() {
            return Ok(false);
        };
        if node.is_accessible(ctx).await.is_ok() {
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
    node: &mut BackgroundNodeClient,
    wait_until_ready: bool,
) -> Result<bool> {
    let node_name = node.node_name();
    let retries = FibonacciBackoff::from_millis(IS_NODE_READY_TIME_BETWEEN_CHECKS_MS);
    let now = std::time::Instant::now();
    let mut total_time = Duration::from_secs(0);
    for timeout_duration in retries {
        if total_time >= IS_NODE_READY_TIMEOUT || !wait_until_ready && !total_time.is_zero() {
            return Ok(false);
        };
        // Test if node is ready
        // If the node is down, we expect it won't reply and the timeout will trigger the next loop
        let result = node
            .ask_with_timeout::<(), NodeStatus>(ctx, api::query_status(), timeout_duration)
            .await;
        if let Ok(node_status) = result {
            if node_status.status.is_running() {
                let elapsed = now.elapsed();
                info!(%node_name, ?elapsed, "node is ready {:?}", node_status);
                return Ok(true);
            } else {
                trace!(%node_name, "node is initializing");
            }
        } else {
            trace!(%node_name, "node is initializing");
        }
        total_time = total_time.add(timeout_duration)
    }
    Ok(false)
}
