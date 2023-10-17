use crate::util::local_cmd;
use crate::{docs, fmt_ok, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam_api::cli_state::traits::StateDirTrait;

const LONG_ABOUT: &str = include_str!("./static/default/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/default/after_long_help.txt");

/// Change the default vault
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the vault to be set as default
    name: String,
}

impl DefaultCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DefaultCommand) -> miette::Result<()> {
    let DefaultCommand { name } = cmd;
    let state = opts.state.vaults;
    let v = state.get(&name)?;
    // If it exists, warn the user and exit
    if state.is_default(v.name())? {
        Err(miette!("The vault '{}' is already the default", name))
    }
    // Otherwise, set it as default
    else {
        state.set_default(v.name())?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!("The vault '{name}' is now the default"))
            .machine(&name)
            .json(serde_json::json!({ "name": name }))
            .write_line()?;
        Ok(())
    }
}
