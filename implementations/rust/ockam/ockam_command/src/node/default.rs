use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam_node::Context;

use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/default/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/default/after_long_help.txt");

/// Change the default node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the node to set as default
    node_name: Option<String>,
}

impl DefaultCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _cxt: Context,
    (opts, cmd): (CommandGlobalOpts, DefaultCommand),
) -> miette::Result<()> {
    if let Some(node_name) = cmd.node_name {
        if opts.state.is_default_node(&node_name).await? {
            return Err(miette!("The node '{node_name}' is already the default"));
        } else {
            opts.state.set_default_node(&node_name).await?;
            opts.terminal
                .stdout()
                .plain(fmt_ok!("The node '{node_name}' is now the default"))
                .machine(&node_name)
                .write_line()?;
        }
    } else {
        let default_node_name = opts.state.get_default_node_name().await?;
        let _ = opts
            .terminal
            .stdout()
            .plain(fmt_ok!("The default node is '{default_node_name}'"))
            .write_line();
    }
    Ok(())
}
