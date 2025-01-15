use async_trait::async_trait;
use std::time::Duration;

use clap::Args;
use console::Term;
use miette::IntoDiagnostic;
use ockam_api::nodes::models::node::NodeResources;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_api::CliState;
use ockam_core::TryClone;
use ockam_node::Context;

use crate::terminal::tui::ShowCommandTui;
use crate::tui::PluralTerm;
use crate::util::api;
use crate::{docs, Command, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of a node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// The name of the node from which to fetch the details.
    /// If not provided, the default node is used.
    node_name: Option<String>,
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "node show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
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
            ctx: ctx.try_clone().into_diagnostic()?,
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
        let node_resources = get_node_resources(&self.ctx, &self.opts.state, &mut node).await?;
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
) -> miette::Result<NodeResources> {
    if let Ok(resources) = node
        .ask_with_timeout(ctx, api::get_node_resources(), Duration::from_secs(1))
        .await
    {
        return Ok(resources);
    }

    let node_info = cli_state.get_node(node.node_name()).await?;
    let identity = cli_state
        .get_named_identity_by_identifier(&node_info.identifier())
        .await?;
    NodeResources::empty(node_info, identity.name()).into_diagnostic()
}
