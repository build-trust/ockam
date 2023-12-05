use clap::Args;
use colorful::Colorful;
use console::Term;
use ockam_node::Context;

use crate::terminal::tui::DeleteCommandTui;
use crate::terminal::PluralTerm;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts, Terminal, TerminalStream};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete nodes
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the node to be deleted
    #[arg(group = "nodes")]
    node_name: Option<String>,

    /// Terminate all node processes and delete all node configurations
    #[arg(long, short, group = "nodes")]
    all: bool,

    /// Terminate node process(es) immediately (uses SIGKILL instead of SIGTERM)
    #[arg(display_order = 901, long, short)]
    force: bool,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    DeleteTui::run(opts, cmd).await
}

pub struct DeleteTui {
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
}

impl DeleteTui {
    pub async fn run(opts: CommandGlobalOpts, cmd: DeleteCommand) -> miette::Result<()> {
        let tui = Self { opts, cmd };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Node;

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.cmd.node_name.as_deref()
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
        Ok(self
            .opts
            .state
            .get_node_or_default(&self.cmd.node_name)
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

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        self.opts
            .state
            .delete_node(item_name, self.cmd.force)
            .await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Node with name {} has been deleted",
                item_name.light_magenta()
            ))
            .machine(item_name)
            .json(serde_json::json!({ "name": &item_name }))
            .write_line()?;
        Ok(())
    }
}
