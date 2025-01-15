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
use ockam_core::TryClone;
use ockam_node::Context;

use crate::tui::{DeleteCommandTui, PluralTerm};
use crate::{node::NodeOpts, Command, CommandGlobalOpts};

/// Delete a Kafka Outlet
#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// Kafka outlet service address
    pub address: Option<String>,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    pub(crate) yes: bool,

    /// Delete all the Kafka Outlets
    #[arg(long)]
    pub(crate) all: bool,
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "kafka-outlet delete";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(DeleteTui::run(ctx, opts, self).await?)
    }
}

#[derive(TryClone)]
struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node: BackgroundNodeClient,
    cmd: DeleteCommand,
}

impl DeleteTui {
    pub async fn run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.node_opts.at_node).await?;
        let tui = Self {
            ctx: ctx.try_clone()?,
            opts,
            node,
            cmd,
        };
        tui.delete().await
    }
}

#[async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::KafkaOutlet;

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
        let outlets: Vec<ServiceStatus> = self
            .node
            .ask(
                &self.ctx,
                Request::get(format!("/node/services/{}", DefaultAddress::KAFKA_OUTLET)),
            )
            .await?;
        let addresses = outlets.into_iter().map(|i| i.addr).collect();
        Ok(addresses)
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        self.node
            .tell(
                &self.ctx,
                Request::delete(format!("/node/services/{}", DefaultAddress::KAFKA_OUTLET))
                    .body(DeleteServiceRequest::new(item_name)),
            )
            .await?;
        let node_name = self.node.node_name();
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Kafka Outlet with address {} on Node {} has been deleted",
                color_primary(item_name),
                color_primary(node_name)
            ))
            .json(serde_json::json!({ "address": item_name, "node": node_name }))
            .write_line()?;
        Ok(())
    }
}
