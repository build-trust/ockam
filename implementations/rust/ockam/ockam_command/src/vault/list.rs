use clap::Args;
use miette::IntoDiagnostic;

use ockam_api::cli_state::traits::StateDirTrait;

use crate::util::local_cmd;
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
    pub fn run(self, opts: CommandGlobalOpts) {
        local_cmd(run_impl(opts));
    }
}

fn run_impl(opts: CommandGlobalOpts) -> miette::Result<()> {
    let vaults = opts.state.vaults.list()?;
    let plain = opts
        .terminal
        .build_list(&vaults, "Vaults", "No vaults found on this system.")?;

    let json = serde_json::to_string_pretty(&vaults).into_diagnostic()?;

    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;

    Ok(())
}
