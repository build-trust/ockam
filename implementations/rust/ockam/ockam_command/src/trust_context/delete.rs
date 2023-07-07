use clap::Args;
use colorful::Colorful;

use ockam_api::cli_state::traits::StateDirTrait;

use crate::terminal::ConfirmResult;
use crate::util::local_cmd;
use crate::{docs, fmt_ok, CommandGlobalOpts};

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
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> miette::Result<()> {
    let DeleteCommand { name } = cmd;
    let state = opts.state.trust_contexts;
    state.get(&name)?;
    if let ConfirmResult::No = opts
        .terminal
        .confirm("Are you sure you want to delete this trust context?")?
    {
        // If the user has not confirmed, exit
        return Ok(());
    }
    state.delete(&name)?;
    opts.terminal
        .stdout()
        .plain(fmt_ok!(
            "The trust context with name '{name}' has been deleted"
        ))
        .machine(&name)
        .json(serde_json::json!({ "trust-context": { "name": &name } }))
        .write_line()?;
    Ok(())
}
