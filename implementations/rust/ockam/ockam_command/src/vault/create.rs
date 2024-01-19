use clap::Args;
use colorful::Colorful;
use std::path::PathBuf;

use crate::util::async_cmd;
use crate::{docs, fmt_info, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a vault
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    #[arg()]
    pub name: Option<String>,

    #[arg(long)]
    pub path: Option<PathBuf>,

    #[arg(long, default_value = "false")]
    pub aws_kms: bool,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |_ctx| async move {
            self.async_run(opts).await
        })
    }

    pub fn name(&self) -> String {
        "create vault".into()
    }

    pub(crate) async fn async_run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        if opts.state.get_named_vaults().await?.is_empty() {
            opts.terminal.write_line(&fmt_info!(
            "This is the first vault to be created in this environment. It will be set as the default vault"
        ))?;
        }
        let vault = if self.aws_kms {
            opts.state.create_kms_vault(&self.name, &self.path).await?
        } else {
            opts.state
                .create_named_vault(&self.name, &self.path)
                .await?
        };

        opts.terminal
            .stdout()
            .plain(fmt_ok!("Vault created with name '{}'!", vault.name()))
            .machine(vault.name())
            .json(serde_json::json!({ "name": &self.name }))
            .write_line()?;
        Ok(())
    }
}
