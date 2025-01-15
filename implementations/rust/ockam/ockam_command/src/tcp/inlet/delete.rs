use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use console::Term;

use crate::node::NodeOpts;
use crate::tcp::util::alias_parser;
use crate::{docs, Command, CommandGlobalOpts};
use ockam::Context;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::service::tcp_inlets::Inlets;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::api::Request;
use ockam_core::TryClone;

use crate::terminal::tui::DeleteCommandTui;
use crate::tui::PluralTerm;

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP Inlet
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    /// Delete the inlet with this alias
    #[arg(display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,

    /// Node on which to stop the tcp inlet. If none are provided, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,

    /// Delete all the TCP Inlets
    #[arg(long)]
    all: bool,
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "tcp-inlet delete";

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

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::TcpInlet;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.alias.clone()
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
        let inlets: Vec<InletStatus> = self
            .node
            .ask(&self.ctx, Request::get("/node/inlet"))
            .await?;
        let names = inlets.into_iter().map(|i| i.alias).collect();
        Ok(names)
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        let node_name = self.node.node_name();
        self.node.delete_inlet(&self.ctx, item_name).await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "TCP Inlet with alias {} on Node {} has been deleted",
                color_primary(item_name),
                color_primary(node_name)
            ))
            .json(serde_json::json!({ "alias": item_name, "node": node_name }))
            .write_line()?;
        Ok(())
    }
}
