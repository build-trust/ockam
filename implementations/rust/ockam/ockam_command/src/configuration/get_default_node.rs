use clap::Args;

use crate::CommandGlobalOpts;
use crate::util::local_cmd;

#[derive(Clone, Debug, Args)]
pub struct GetDefaultNodeCommand {}

impl GetDefaultNodeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        local_cmd(run_impl(options));
    }
}

fn run_impl(_opts: CommandGlobalOpts) -> miette::Result<()> {
    // TODO: get from opts.state.nodes().default()
    todo!()
}
