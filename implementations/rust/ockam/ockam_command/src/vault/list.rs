use clap::Args;
use miette::IntoDiagnostic;

use crate::vault::util::VaultOutput;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List vaults
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand;

impl ListCommand {
    pub fn name(&self) -> String {
        "vaults list".into()
    }

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let vaults = opts
            .state
            .get_named_vaults()
            .await?
            .into_iter()
            .map(|v| VaultOutput::new(&v))
            .collect::<Vec<_>>();
        let plain = opts.terminal.build_list(&vaults, "No Vaults found")?;
        let json = serde_json::to_string(&vaults).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
        Ok(())
    }
}
