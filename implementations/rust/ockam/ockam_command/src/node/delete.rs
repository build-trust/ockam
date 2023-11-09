use clap::Args;
use colorful::Colorful;
use console::Term;
use ockam_api::cli_state::StateDirTrait;
use ockam_node::Context;

use crate::node::get_node_name;
use crate::terminal::tui::DeleteCommandTui;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, fmt_warn, CommandGlobalOpts, Terminal, TerminalStream};

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
    NodeDeleteTui::run(opts, cmd).await
}

pub struct NodeDeleteTui {
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
}

impl NodeDeleteTui {
    pub async fn run(opts: CommandGlobalOpts, cmd: DeleteCommand) -> miette::Result<()> {
        let tui = Self { opts, cmd };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl DeleteCommandTui for NodeDeleteTui {
    const ITEM_NAME: &'static str = "nodes";

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

    fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self.opts.state.nodes.list_items_names()?)
    }

    async fn delete_single(&self) -> miette::Result<()> {
        let node_name = get_node_name(&self.opts.state, &self.cmd.node_name);
        self.opts
            .state
            .nodes
            .delete_sigkill(&node_name, self.cmd.force)?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!("Node with name '{node_name}' has been deleted"))
            .machine(&node_name)
            .json(serde_json::json!({ "name": &node_name }))
            .write_line()?;
        Ok(())
    }

    async fn delete_multiple(&self, selected_items_names: Vec<String>) -> miette::Result<()> {
        let plain = selected_items_names
            .iter()
            .map(|name| {
                if self
                    .opts
                    .state
                    .nodes
                    .delete_sigkill(name, self.cmd.force)
                    .is_ok()
                {
                    fmt_ok!("Node '{name}' deleted\n")
                } else {
                    fmt_warn!("Failed to delete node '{name}'\n")
                }
            })
            .collect::<String>();
        self.terminal().stdout().plain(plain).write_line()?;
        Ok(())
    }
}
