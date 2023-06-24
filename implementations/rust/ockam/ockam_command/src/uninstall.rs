use crate::util::installer::uninstall;
use clap::Args;
use colorful::Colorful;
use miette::miette;

use crate::{fmt_info, terminal::ConfirmResult, util::local_cmd, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    subcommand_required = false,
    long_about = "Uninstall ockam"
)]
pub struct UninstallCommand {
    #[arg(long, short)]
    yes: bool,
}

impl UninstallCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: UninstallCommand) -> miette::Result<()> {
    if !cmd.yes {
        match opts
            .terminal
            .confirm(&fmt_info!("Are you sure you wish to uninstall ockam?"))?
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
    opts.terminal
        .write_line(fmt_info!("Uninstalling ockam..."))?;
    uninstall()?;
    opts.terminal
        .stdout()
        .plain(fmt_info!("Uninstalled ockam"))
        .write_line()?;
    Ok(())
}
