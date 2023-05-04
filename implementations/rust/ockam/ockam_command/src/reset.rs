use crate::terminal::ConfirmResult;
use crate::{fmt_ok, CommandGlobalOpts};
use anyhow::anyhow;
use clap::Args;
use colorful::Colorful;

/// Full Ockam Reset
#[derive(Clone, Debug, Args)]
pub struct ResetCommand {
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl ResetCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: ResetCommand) -> crate::Result<()> {
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
                return Err(anyhow!("Use --yes to confirm").into());
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
