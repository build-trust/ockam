use crate::terminal::tui::DeleteCommandTui;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, fmt_warn, CommandGlobalOpts, Terminal, TerminalStream};
use clap::Args;
use colorful::Colorful;

use console::Term;
use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;

use super::get_identity_name;

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete an identity
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the identity to be deleted
    name: Option<String>,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,

    #[arg(long, short)]
    all: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
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

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    DeleteTui::run(opts, cmd).await
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: &'static str = "identity";
    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.cmd.name.as_deref()
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
        Ok(get_identity_name(&self.opts.state, &self.cmd.name))
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self.opts.state.identities.list_items_names()?)
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        let state = &self.opts.state;
        let idt = state.identities.get(item_name)?;
        state.delete_identity(idt)?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "The identity named '{}' has been deleted",
                item_name
            ))
            .machine(item_name)
            .json(serde_json::json!({ "name": item_name }))
            .write_line()?;
        Ok(())
    }

    async fn delete_multiple(&self, selected_items_names: Vec<String>) -> miette::Result<()> {
        let plain = selected_items_names
            .iter()
            .map(|name| {
                let idt = self.opts.state.identities.get(name)?;
                if self.opts.state.delete_identity(idt).is_ok() {
                    Ok(fmt_ok!("Identity '{name}' deleted\n"))
                } else {
                    Ok(fmt_warn!("Failed to delete identity '{name}'\n"))
                }
            })
            .collect::<miette::Result<String>>()?;
        self.terminal().stdout().plain(plain).write_line()?;
        Ok(())
    }
}
