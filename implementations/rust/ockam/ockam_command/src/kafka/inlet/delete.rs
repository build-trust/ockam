use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use console::Term;
use ockam_api::colors::color_primary;
use ockam_api::{fmt_ok, DefaultAddress};

use ockam_api::nodes::models::services::{DeleteServiceRequest, ServiceStatus};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::api::Request;
use ockam_node::Context;

use crate::tui::{DeleteCommandTui, PluralTerm};
use crate::{docs, node::NodeOpts, Command, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Kafka Inlet
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// Kafka Inlet service address
    pub address: Option<String>,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    pub(crate) yes: bool,

    /// Delete all the Kafka Inlets
    #[arg(long)]
    pub(crate) all: bool,
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "kafka-inlet delete";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(DeleteTui::run(ctx, opts, &self).await?)
    }
}

struct DeleteTui<'a> {
    ctx: &'a Context,
    opts: CommandGlobalOpts,
    node: BackgroundNodeClient,
    cmd: &'a DeleteCommand,
}

impl<'a> DeleteTui<'a> {
    pub async fn run(
        ctx: &'a Context,
        opts: CommandGlobalOpts,
        cmd: &'a DeleteCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.node_opts.at_node).await?;
        let tui = Self {
            ctx,
            opts,
            node,
            cmd,
        };
        tui.delete().await
    }
}

#[async_trait]
impl<'a> DeleteCommandTui for DeleteTui<'a> {
    const ITEM_NAME: PluralTerm = PluralTerm::KafkaInlet;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.address.clone()
    }

    fn cmd_arg_delete_all(&self) -> bool {
        self.cmd.all
    }

    fn cmd_arg_confirm_deletion(&self) -> bool {
        self.cmd.yes
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let inlets: Vec<ServiceStatus> = self
            .node
            .ask(
                self.ctx,
                Request::get(format!("/node/services/{}", DefaultAddress::KAFKA_INLET)),
            )
            .await?;
        let addresses = inlets.into_iter().map(|i| i.addr).collect();
        Ok(addresses)
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        self.node
            .tell(
                self.ctx,
                Request::delete(format!("/node/services/{}", DefaultAddress::KAFKA_INLET))
                    .body(DeleteServiceRequest::new(item_name)),
            )
            .await?;
        let node_name = self.node.node_name();
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Kafka Inlet with address {} on Node {} has been deleted",
                color_primary(item_name),
                color_primary(&node_name)
            ))
            .json(serde_json::json!({ "address": item_name, "node": node_name }))
            .write_line()?;
        Ok(())
    }
}
