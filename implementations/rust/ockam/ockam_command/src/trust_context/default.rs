use crate::{docs, fmt_ok, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::CliStateError;

const LONG_ABOUT: &str = include_str!("./static/default/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/default/after_long_help.txt");

/// Change the default trust context
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the trust context to be set as default
    name: String,
}

impl DefaultCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{e:?}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DefaultCommand) -> crate::Result<()> {
    let DefaultCommand { name } = cmd;
    let state = opts.state.trust_contexts;
    match state.get(&name) {
        Ok(v) => {
            // If it exists, warn the user and exit
            if state.is_default(v.name())? {
                Err(miette!("Trust context '{name}' is already the default").into())
            }
            // Otherwise, set it as default
            else {
                state.set_default(v.name())?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("Trust context '{name}' is now the default"))
                    .machine(&name)
                    .json(serde_json::json!({ "trust-context": {"name": name} }))
                    .write_line()?;
                Ok(())
            }
        }
        Err(err) => match err {
            CliStateError::NotFound => Err(miette!("Trust context '{name}' not found").into()),
            _ => Err(err.into()),
        },
    }
}
