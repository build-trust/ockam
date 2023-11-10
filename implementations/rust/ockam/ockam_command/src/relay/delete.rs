use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::miette;

use ockam::Context;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::node::get_node_name;
use crate::relay::util::relay_name_parser;
use crate::terminal::tui::DeleteCommandTui;
use crate::util::{node_rpc, parse_node_name};
use crate::{docs, fmt_ok, fmt_warn, CommandGlobalOpts, Terminal, TerminalStream};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Relay
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    /// Name assigned to the Relay, prefixed with 'forward_to_'. Example: 'forward_to_myrelay'
    #[arg(value_parser = relay_name_parser)]
    relay_name: Option<String>,

    /// Node on which to delete the Relay. If not provided, the default node will be used
    #[arg(global = true, long, value_name = "NODE")]
    pub at: Option<String>,

    /// Confirm the deletion without prompting
    #[arg(long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
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
        let node_name = {
            let name = get_node_name(&opts.state, &cmd.at);
            parse_node_name(&name)?
        };
        let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
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
    const ITEM_NAME: &'static str = "relay";

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.cmd.relay_name.as_deref()
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
        self.cmd
            .relay_name
            .clone()
            .ok_or(miette!("No relay name provided"))
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let relays: Vec<RelayInfo> = self
            .node
            .ask(&self.ctx, Request::get("/node/forwarder"))
            .await?;
        let names = relays
            .into_iter()
            .map(|i| i.remote_address().to_string())
            .collect();
        Ok(names)
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        let node_name = self.node.node_name();
        self.node
            .tell(
                &self.ctx,
                Request::delete(format!("/node/forwarder/{item_name}")),
            )
            .await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Relay with name {} on Node {} has been deleted",
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
            let res = self
                .node
                .tell(
                    &self.ctx,
                    Request::delete(format!("/node/forwarder/{item_name}")),
                )
                .await;
            if res.is_ok() {
                plain.push_str(&fmt_ok!(
                    "Relay with name {} on Node {} has been deleted\n",
                    item_name.light_magenta(),
                    node_name.light_magenta()
                ));
            } else {
                plain.push_str(&fmt_warn!(
                    "Failed to delete relay with name {} on Node {}\n",
                    item_name.light_magenta(),
                    node_name.light_magenta()
                ));
            }
        }
        self.terminal().stdout().plain(plain).write_line()?;
        Ok(())
    }
}
