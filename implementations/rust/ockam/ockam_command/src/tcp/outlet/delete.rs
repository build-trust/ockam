use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::miette;

use ockam::Context;
use ockam_api::nodes::models::portal::OutletList;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;

use crate::node::NodeOpts;
use crate::tcp::util::alias_parser;
use crate::terminal::tui::DeleteCommandTui;
use crate::terminal::PluralTerm;
use crate::util::node_rpc;
use crate::{color, docs, fmt_ok, CommandGlobalOpts, OckamColor, Terminal, TerminalStream};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP Outlet
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    /// Delete the outlet with this alias
    #[arg(display_order = 900,  id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,

    /// Node on which to stop the tcp outlet. If none are provided, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,

    #[arg(long, short, group = "tcp-outlets")]
    all: bool,
}

pub struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
    node: BackgroundNodeClient,
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
            cmd,
            node,
        };
        tui.delete().await
    }
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self))
    }
}

pub async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    DeleteTui::run(ctx, opts, cmd).await
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Outlet;

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.cmd.alias.as_deref()
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

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        self.cmd.alias.clone().ok_or(miette!("No alias provided"))
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let res: OutletList = self
            .node
            .ask(&self.ctx, Request::get("/node/outlet"))
            .await?;
        let items_names: Vec<String> = res.list.iter().map(|outlet| outlet.alias.clone()).collect();
        Ok(items_names)
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        let node_name = self.node.node_name();
        self.node
            .tell(
                &self.ctx,
                Request::delete(format!("/node/outlet/{item_name}")),
            )
            .await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Outlet with alias {} on Node {} has been deleted",
                color!(item_name, OckamColor::PrimaryResource),
                color!(node_name, OckamColor::PrimaryResource)
            ))
            .machine(item_name)
            .json(serde_json::json!({ "alias": item_name, "node": node_name }))
            .write_line()
            .unwrap();
        Ok(())
    }
}
