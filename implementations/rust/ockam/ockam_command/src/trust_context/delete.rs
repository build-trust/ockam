use clap::Args;
use colorful::Colorful;
use ockam_node::Context;

use crate::util::node_rpc;
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

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    if opts.terminal.confirmed_with_flag_or_prompt(
        cmd.yes,
        "Are you sure you want to delete this trust context?",
    )? {
        let name = &cmd.name;
        opts.state.delete_trust_context(name).await?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "The trust context with name '{name}' has been deleted"
            ))
            .machine(name)
            .json(serde_json::json!({ "name": &name }))
            .write_line()?;
    }
    Ok(())
}
