use std::path::PathBuf;

use clap::Args;
use colorful::Colorful;
use ockam_api::{fmt_err, fmt_info};

use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/move/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/move/after_long_help.txt");

/// Move a vault to a different path
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct MoveCommand {
    #[arg()]
    name: String,

    #[arg(long)]
    path: PathBuf,
}

impl MoveCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |_ctx| async move {
            self.async_run(opts).await
        })
    }

    pub fn name(&self) -> String {
        "move vault".into()
    }

    async fn async_run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let vault_name = self.name.clone();
        let vault_path = self.path.clone();
        match opts
            .state
            .move_vault(&vault_name, &vault_path.clone())
            .await
        {
            Ok(()) => opts
                .terminal
                .write_line(fmt_info!("Moved the vault {vault_name} to {vault_path:?}"))?,
            Err(e) => {
                opts.terminal.write_line(fmt_err!(
                    "Could not move the vault {vault_name} to {vault_path:?}: {e:?}"
                ))?;
                return Err(e)?;
            }
        };
        Ok(())
    }
}
