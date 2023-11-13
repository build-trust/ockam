use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam_node::Context;

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
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DefaultCommand),
) -> miette::Result<()> {
    let name = cmd.name;
    let default_trust_context = opts.state.get_default_trust_context().await?;
    // If it exists, warn the user and exit
    if default_trust_context.name() == name {
        Err(miette!("The trust context '{name}' is already the default"))
    }
    // Otherwise, set it as default
    else {
        opts.state.set_default_trust_context(&name).await?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!("The trust context '{name}' is now the default"))
            .machine(&name)
            .json(serde_json::json!({"name": name}))
            .write_line()?;
        Ok(())
    }
}
