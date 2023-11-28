use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::miette;

use ockam::Context;
use ockam_api::nodes::models::portal::InletList;
use ockam_api::nodes::service::portals::Inlets;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::fmt_ok;
use crate::node::NodeOpts;
use crate::tcp::util::alias_parser;
use crate::terminal::tui::DeleteCommandTui;
use crate::util::node_rpc;
use crate::{docs, fmt_warn, CommandGlobalOpts, Terminal, TerminalStream};

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

struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node: BackgroundNode,
    cmd: DeleteCommand,
}

impl DeleteTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNode::create(&ctx, &opts.state, &cmd.node_opts.at_node).await?;
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
    const ITEM_NAME: &'static str = "inlet";

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.cmd.alias.as_deref()
    }

    fn cmd_arg_delete_all(&self) -> bool {
        false
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
        let inlets: InletList = self
            .node
            .ask(&self.ctx, Request::get("/node/inlet"))
            .await?;
        let names = inlets.list.into_iter().map(|i| i.alias).collect();
        Ok(names)
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        let node_name = self.node.node_name();
        self.node.delete_inlet(&self.ctx, item_name).await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "TCP inlet with alias {} on Node {} has been deleted",
                item_name.light_magenta(),
                node_name.light_magenta()
            ))
            .write_line()?;
        Ok(())
    }

    async fn delete_multiple(&self, items_names: Vec<String>) -> miette::Result<()> {
        let node_name = self.node.node_name();
        let mut plain = String::new();
        for item_name in items_names {
            if self.node.delete_inlet(&self.ctx, &item_name).await.is_ok() {
                plain.push_str(&fmt_ok!(
                    "TCP inlet with alias {} on Node {} has been deleted\n",
                    item_name.light_magenta(),
                    node_name.clone().light_magenta()
                ));
            } else {
                plain.push_str(&fmt_warn!(
                    "Failed to delete TCP inlet with alias {} on Node {}\n",
                    item_name.light_magenta(),
                    node_name.clone().light_magenta()
                ));
            }
        }
        self.terminal().stdout().plain(plain).write_line()?;
        Ok(())
    }
}
