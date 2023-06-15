use crate::terminal::ConfirmResult;
use crate::util::local_cmd;
use crate::{fmt_ok, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use miette::miette;

/// Removes the local Ockam configuration including all Identities and Nodes
#[derive(Clone, Debug, Args)]
pub struct ResetCommand {
    /// Confirm the reset without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl ResetCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: ResetCommand) -> miette::Result<()> {
    if !cmd.yes {
        match opts
            .terminal
            .confirm("This will delete the local Ockam configuration. Are you sure?")?
        {
            ConfirmResult::Yes => {}
            ConfirmResult::No => {
                return Ok(());
            }
            ConfirmResult::NonTTY => {
                return Err(miette!("Use --yes to confirm"));
            }
        }
    }
    opts.state.delete(true)?;
    opts.terminal
        .stdout()
        .plain(fmt_ok!("Local Ockam configuration deleted"))
        .write_line()?;
    Ok(())
}
