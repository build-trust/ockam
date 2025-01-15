use crate::{docs, CommandGlobalOpts};
use clap::Args;
use console::Term;
use miette::IntoDiagnostic;
use ockam_api::terminal::{Terminal, TerminalStream};

use ockam_api::output::Output;

use crate::terminal::tui::ShowCommandTui;
use crate::tui::PluralTerm;
use crate::vault::util::VaultOutput;

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of a vault
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name of the vault
    pub name: Option<String>,
}

impl ShowCommand {
    pub fn name(&self) -> String {
        "vault show".into()
    }

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        ShowTui::run(opts, self.clone()).await
    }
}

pub struct ShowTui {
    opts: CommandGlobalOpts,
    vault_name: Option<String>,
}

impl ShowTui {
    pub async fn run(opts: CommandGlobalOpts, cmd: ShowCommand) -> miette::Result<()> {
        let tui = Self {
            opts,
            vault_name: cmd.name,
        };
        tui.show().await
    }
}

#[ockam_core::async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Vault;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.vault_name.clone()
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        match &self.vault_name {
            Some(vault_name) => Ok(vault_name.clone()),
            None => Ok(self
                .opts
                .state
                .get_or_create_default_named_vault()
                .await?
                .name()),
        }
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

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let vault = VaultOutput::new(&self.opts.state.get_named_vault(item_name).await?);
        self.terminal()
            .stdout()
            .plain(vault.item()?)
            .json(serde_json::to_string(&vault).into_diagnostic()?)
            .machine(vault.name())
            .write_line()?;
        Ok(())
    }
}
