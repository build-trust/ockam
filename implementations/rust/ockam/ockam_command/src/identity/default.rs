use crate::util::local_cmd;
use crate::{docs, fmt_ok, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam_api::cli_state::traits::StateDirTrait;

const LONG_ABOUT: &str = include_str!("./static/default/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/default/after_long_help.txt");

/// Change the default identity
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the identity to be set as default
    name: String,
}

impl DefaultCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        local_cmd(run_impl(options, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DefaultCommand) -> miette::Result<()> {
    let state = opts.state.identities;
    let idt = state.get(&cmd.name)?;
    // If it exists, warn the user and exit
    if state.is_default(idt.name())? {
        Err(miette!(
            "The identity '{}' is already the default",
            &cmd.name
        ))
    }
    // Otherwise, set it as default
    else {
        state.set_default(idt.name())?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!("The identity '{}' is now the default", &cmd.name))
            .machine(&cmd.name)
            .write_line()?;
        Ok(())
    }
}
