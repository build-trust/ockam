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
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::api::Request;
use ockam_core::TryClone;

use crate::terminal::tui::DeleteCommandTui;
use crate::tui::PluralTerm;

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");
const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");

/// Delete a TCP Outlet
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Delete the Outlet with this alias name. If you don't provide an alias, you will be
    /// prompted to select from a list of available Outlets to delete
    #[arg(display_order = 900,  id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,

    /// Node on which to stop the TCP Outlet. If you don't provide it, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Run the delete command, without prompting for confirmation. This is useful for
    /// scripts
    #[arg(display_order = 901, long, short)]
    yes: bool,

    /// Delete all the TCP Outlets
    #[arg(long, group = "tcp-outlets")]
    all: bool,
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "tcp-outlet delete";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(DeleteTui::run(ctx, opts, self).await?)
    }
}

#[derive(TryClone)]
pub struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
    node: BackgroundNodeClient,
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
            cmd,
            node,
        };
        tui.delete().await
    }
}

#[async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::TcpOutlet;

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
        let res: Vec<OutletStatus> = self
            .node
            .ask(&self.ctx, Request::get("/node/outlet"))
            .await?;
        let items_names: Vec<String> = res
            .iter()
            .map(|outlet| outlet.worker_addr.address().to_string())
            .collect();
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
                "TCP Outlet with alias {} on node {} has been deleted",
                color_primary(item_name),
                color_primary(&node_name)
            ))
            .machine(item_name)
            .json(serde_json::json!({ "alias": item_name, "node": node_name }))
            .write_line()
            .unwrap();
        Ok(())
    }
}
