use crate::CommandGlobalOpts;
use anyhow::anyhow;
use clap::Args;
use ockam_api::cli_state::CliStateError;

/// Set the default identity
#[derive(Clone, Debug, Args)]
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
            if state.is_default(&idt.name)? {
                Err(anyhow!("Identity '{}' is already the default", &cmd.name).into())
            }
            // Otherwise, set it as default
            else {
                state.set_default(&idt.name)?;
                println!("Identity '{}' is now the default", &cmd.name,);
                Ok(())
            }
        }
        Err(err) => match err {
            CliStateError::NotFound(_) => Err(anyhow!("Identity '{}' not found", &cmd.name).into()),
            _ => Err(err.into()),
        },
    }
}
