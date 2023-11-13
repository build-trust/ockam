use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("./static/default/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/default/after_long_help.txt");

/// Change the default vault
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the vault to be set as default
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
    // If the vault is already the default vault, warn the user and exit
    if opts
        .state
        .get_named_vault(&cmd.name)
        .await
        .ok()
        .map(|v| v.is_default())
        .unwrap_or(false)
    {
        Err(miette!("The vault '{}' is already the default", &cmd.name))
    }
    // Otherwise, set it as default
    else {
        opts.state.set_default_vault(&cmd.name).await?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!("The vault '{}' is now the default", &cmd.name))
            .machine(&cmd.name)
            .json(serde_json::json!({ "name": cmd.name }))
            .write_line()?;
        Ok(())
    }
}
