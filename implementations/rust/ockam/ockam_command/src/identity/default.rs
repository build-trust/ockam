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
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the identity to be set as default
    name: Option<String>,
}

impl DefaultCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        local_cmd(run_impl(options, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DefaultCommand) -> miette::Result<()> {
    if let Some(name) = cmd.name {
        let state = opts.state.identities;
        let idt = state.get(&name)?;
        // If it's already the default, warn the user and exit
        if state.is_default(idt.name())? {
            Err(miette!(
                "The identity named '{}' is already the default",
                &name
            ))
        }
        // Otherwise, set it as default
        else {
            state.set_default(idt.name())?;
            opts.terminal
                .stdout()
                .plain(fmt_ok!("The identity named '{}' is now the default", &name))
                .machine(&name)
                .write_line()?;
            Ok(())
        }
    }
    // No argument provided, show default identity name
    else {
        let state = opts.state.identities.get_or_default(None)?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "The name of the default identity is '{}'",
                state.name()
            ))
            .write_line()?;
        Ok(())
    }
}
