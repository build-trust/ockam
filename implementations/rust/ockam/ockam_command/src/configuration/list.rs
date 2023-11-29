use clap::Args;

use ockam_node::Context;

use crate::util::node_rpc;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, options);
    }
}

async fn run_impl(_ctx: Context, opts: CommandGlobalOpts) -> miette::Result<()> {
    for node in opts.state.get_nodes().await? {
        opts.terminal.write(format!("Node: {}\n", node.name()))?;
    }
    Ok(())
}
