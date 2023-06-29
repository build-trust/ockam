use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam_api::cli_state::StateDirTrait;

use crate::{CommandGlobalOpts, docs, fmt_ok};
use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::local_cmd;

const LONG_ABOUT: &str = include_str!("./static/default/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/default/after_long_help.txt");

/// Change the default node
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the node.
    #[arg()]
    node_name: Option<String>,
}

impl DefaultCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_name);
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DefaultCommand) -> miette::Result<()> {
    let name = get_node_name(&opts.state, &cmd.node_name);
    if opts.state.nodes.is_default(&name)? {
        Err(miette!("The node '{name}' is already the default"))
    } else {
        opts.state.nodes.set_default(&name)?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!("The node '{name}' is now the default"))
            .machine(&name)
            .write_line()?;
        Ok(())
    }
}
