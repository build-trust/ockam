use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::CliStateError;

use crate::terminal::ConfirmResult;
use crate::{docs, fmt_ok, fmt_warn, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a trust context
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the trust context
    pub name: String,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{e:?}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> crate::Result<()> {
    let DeleteCommand { name } = cmd;
    let state = opts.state.trust_contexts;
    match state.get(&name) {
        // If it exists, proceed
        Ok(_) => {
            if let ConfirmResult::No = opts.terminal.confirm(&fmt_warn!(
                "This will delete the trust context with name '{name}'. Do you want to continue?"
            ))? {
                // If the user has not confirmed, exit
                return Ok(());
            }

            state.delete(&name)?;

            opts.terminal
                .stdout()
                .plain(fmt_ok!("Trust context with name '{name}' has been deleted"))
                .machine(&name)
                .json(serde_json::json!({ "trust-context": { "name": &name } }))
                .write_line()?;

            Ok(())
        }
        // Else, return the appropriate error
        Err(err) => match err {
            CliStateError::NotFound => Err(miette!("Trust context '{name}' not found").into()),
            _ => Err(err.into()),
        },
    }
}
