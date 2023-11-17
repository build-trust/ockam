use clap::Args;
use colorful::Colorful;

use console::Term;
use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;

use crate::terminal::tui::DeleteCommandTui;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, fmt_warn, CommandGlobalOpts, Terminal, TerminalStream};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a vault
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the vault
    pub name: Option<String>,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,

    #[arg(long, short)]
    all: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
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

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    _ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> miette::Result<()> {
    DeleteTui::run(opts, cmd).await
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: &'static str = "vault";
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
        Ok(self
            .cmd
            .name
            .clone()
            .unwrap_or(self.opts.state.vaults.default()?.name().to_string()))
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self.opts.state.vaults.list_items_names()?)
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        self.opts.state.vaults.delete(item_name)?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!("Vault with name '{item_name}' has been deleted"))
            .machine(item_name)
            .json(serde_json::json!({ "name": &item_name }))
            .write_line()?;

        Ok(())
    }

    async fn delete_multiple(&self, selected_items_names: Vec<String>) -> miette::Result<()> {
        let plain = selected_items_names
            .iter()
            .map(|name| {
                if self.opts.state.vaults.delete(name).is_ok() {
                    fmt_ok!("Vault '{name}' deleted\n")
                } else {
                    fmt_warn!("Failed to delete vault '{name}'\n")
                }
            })
            .collect::<String>();
        self.terminal().stdout().plain(plain).write_line()?;
        Ok(())
    }
}
