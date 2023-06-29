use clap::Args;

use crate::CommandGlobalOpts;
use crate::util::local_cmd;

#[derive(Clone, Debug, Args)]
pub struct SetDefaultNodeCommand {
    /// Name of the Node
    pub name: String,
}

impl SetDefaultNodeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        local_cmd(run_impl(&self.name, &options));
    }
}

fn run_impl(_name: &str, _options: &CommandGlobalOpts) -> miette::Result<()> {
    // TODO: add symlink to options.state.defaults().node
    todo!()
}
