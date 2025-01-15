use crate::terminal::tui::DeleteCommandTui;
use crate::tui::PluralTerm;
use crate::util::print_warning_for_deprecated_flag_no_effect;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use console::Term;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::terminal::notification::NotificationHandler;
use ockam_api::terminal::{Terminal, TerminalStream};

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
    #[arg(long, group = "nodes")]
    all: bool,

    /// [DEPRECATED] Terminate node process(es) immediately (uses SIGKILL instead of SIGTERM)
    #[arg(display_order = 901, long, short)]
    force: bool,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn name(&self) -> String {
        "node delete".into()
    }

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        if self.force {
            print_warning_for_deprecated_flag_no_effect(&opts, "--force")?;
        }
        DeleteTui::run(opts, self.clone()).await
    }
}

#[derive(Clone)]
pub struct DeleteTui {
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
}

impl DeleteTui {
    pub async fn run(opts: CommandGlobalOpts, cmd: DeleteCommand) -> miette::Result<()> {
        let _notification_handler = NotificationHandler::start(&opts.state, opts.terminal.clone());
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
        self.opts.state.delete_node(item_name).await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "The node with name {} has been deleted",
                color_primary(item_name)
            ))
            .machine(item_name)
            .json(serde_json::json!({ "name": &item_name }))
            .write_line()?;
        Ok(())
    }

    async fn delete_multiple(&self, items_names: Vec<String>) -> miette::Result<()> {
        self.delete_multiple_async(items_names).await
    }
}
