use clap::Args;

use ockam_api::cli_state::StateDirTrait;

use crate::CommandGlobalOpts;
use crate::util::local_cmd;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        local_cmd(run_impl(options, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, _cmd: ListCommand) -> miette::Result<()> {
    for node in opts.state.nodes.list()? {
        opts.terminal.write(format!("Node: {}\n", node.name()))?;
    }
    Ok(())
}
