use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::IntoDiagnostic;

use crate::{docs, Command, CommandGlobalOpts};
use ockam::Context;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::service::tcp_inlets::Inlets;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_api::{color, fmt_ok};
use ockam_core::api::Request;
use ockam_core::AsyncTryClone;

use crate::node::NodeOpts;
use crate::tcp::util::alias_parser;

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

    /// Delete all the TCP Inlet
    #[arg(long, short, group = "tcp-inlets")]
    all: bool,
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "tcp-inlet delete";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(DeleteTui::run(
            ctx.async_try_clone().await.into_diagnostic()?,
            opts,
            self.clone(),
        )
        .await?)
    }
}

struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node: BackgroundNodeClient,
    cmd: DeleteCommand,
}

impl DeleteTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(&ctx, &opts.state, &cmd.node_opts.at_node).await?;
        let tui = Self {
            ctx,
            opts,
            node,
            cmd,
        };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Inlet;

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
                "TCP inlet with alias {} on Node {} has been deleted",
                color!(item_name, OckamColor::PrimaryResource),
                color!(node_name, OckamColor::PrimaryResource)
            ))
            .write_line()?;
        Ok(())
    }
}
