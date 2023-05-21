use crate::{docs, CommandGlobalOpts};
use clap::Args;
use miette::miette;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::CliStateError;

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
        if let Err(e) = run_impl(options, self) {
            eprintln!("{e:?}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DefaultCommand) -> crate::Result<()> {
    let state = opts.state.identities;
    // Check if exists
    match state.get(&cmd.name) {
        Ok(idt) => {
            // If it exists, warn the user and exit
            if state.is_default(idt.name())? {
                Err(miette!("Identity '{}' is already the default", &cmd.name).into())
            }
            // Otherwise, set it as default
            else {
                state.set_default(idt.name())?;
                println!("Identity '{}' is now the default", &cmd.name,);
                Ok(())
            }
        }
        Err(err) => match err {
            CliStateError::NotFound => Err(miette!("Identity '{}' not found", &cmd.name).into()),
            _ => Err(err.into()),
        },
    }
}
