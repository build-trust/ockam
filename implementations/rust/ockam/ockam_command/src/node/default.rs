use crate::node::util::{check_default, set_default_node};
use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::local_cmd;
use crate::{docs, CommandGlobalOpts};
use clap::Args;

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
    let node_name = get_node_name(&opts.state, &cmd.node_name);
    if check_default(&opts, &node_name) {
        println!("Already set to default node");
    } else {
        set_default_node(&opts, &node_name)?;
        println!("Set node '{}' as default", &node_name);
    }
    Ok(())
}
