use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::miette;

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::BackgroundNode;
use ockam_api::nodes::service::relay::Relays;

use crate::relay::util::relay_name_parser;
use crate::terminal::tui::DeleteCommandTui;
use crate::terminal::PluralTerm;
use crate::util::node_rpc;
use crate::{color, docs, fmt_ok, CommandGlobalOpts, OckamColor, Terminal, TerminalStream};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Relay
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    /// Name assigned to the Relay, prefixed with 'forward_to_'. Example: 'forward_to_myrelay'
    #[arg(value_parser = relay_name_parser)]
    relay_name: Option<String>,

    /// Node on which to delete the Relay. If not provided, the default node will be used
    #[arg(global = true, long, value_name = "NODE", value_parser = extract_address_value)]
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
        let node = BackgroundNode::create(&ctx, &opts.state, &cmd.at).await?;
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
    const ITEM_NAME: PluralTerm = PluralTerm::Relay;

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
        let relays = self.node.list_relay(&self.ctx).await?;
        let names = relays
            .into_iter()
            .map(|i| i.remote_address().to_string())
            .collect();
        Ok(names)
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        let node_name = self.node.node_name();
        self.node.delete_relay(&self.ctx, item_name).await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Relay with name {} on Node {} has been deleted",
                color!(item_name, OckamColor::PrimaryResource),
                color!(node_name, OckamColor::PrimaryResource)
            ))
            .write_line()?;
        Ok(())
    }
}
