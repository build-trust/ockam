use clap::Args;
use console::Term;
use miette::IntoDiagnostic;

use ockam_node::Context;

use crate::output::Output;
use crate::terminal::tui::ShowCommandTui;
use crate::terminal::PluralTerm;
use crate::util::node_rpc;
use crate::vault::util::VaultOutput;
use crate::{docs, CommandGlobalOpts, Terminal, TerminalStream};

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
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    ShowTui::run(opts, cmd).await
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

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.vault_name.as_deref()
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
            .plain(vault.output()?)
            .json(serde_json::to_string(&vault).into_diagnostic()?)
            .machine(vault.name())
            .write_line()?;
        Ok(())
    }

    async fn show_multiple(&self, items_names: Vec<String>) -> miette::Result<()> {
        let filtered = self
            .opts
            .state
            .get_named_vaults()
            .await?
            .into_iter()
            .map(|v| VaultOutput::new(&v))
            .filter(|v| items_names.contains(&v.name()))
            .collect::<Vec<_>>();
        let plain = self
            .terminal()
            .build_list(&filtered, "Vaults", "No Vaults found")?;
        let json = serde_json::to_string(&filtered).into_diagnostic()?;
        self.terminal()
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
        Ok(())
    }
}
