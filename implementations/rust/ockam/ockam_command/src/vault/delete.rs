use crate::terminal::tui::DeleteCommandTui;
use crate::tui::PluralTerm;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use console::Term;
use ockam_api::colors::OckamColor;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_api::{color, fmt_ok};

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

    #[arg(long)]
    all: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |_ctx| async move {
            self.async_run(opts).await
        })
    }

    pub fn name(&self) -> String {
        "vault delete".into()
    }

    async fn async_run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
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
        let tui = Self { opts, cmd };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Vault;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.name.clone()
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
            .get_named_vaults()
            .await?
            .iter()
            .map(|v| v.name())
            .collect())
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        self.opts.state.delete_named_vault(item_name).await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Vault with name {} has been deleted",
                color!(item_name, OckamColor::PrimaryResource)
            ))
            .machine(item_name)
            .json(serde_json::json!({ "name": &item_name }))
            .write_line()?;

        Ok(())
    }
}
