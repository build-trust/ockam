use clap::Args;
use colorful::Colorful;
use console::Term;

use crate::terminal::tui::DeleteCommandTui;
use crate::terminal::PluralTerm;
use crate::util::async_cmd;
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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |_ctx| async move {
            self.async_run(opts).await
        })
    }

    pub fn name(&self) -> String {
        "delete node".into()
    }

    async fn async_run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        DeleteTui::run(opts, self.clone()).await
    }
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

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.node_name.clone()
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
