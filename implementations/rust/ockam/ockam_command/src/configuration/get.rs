use crate::util::local_cmd;
use crate::CommandGlobalOpts;
use clap::Args;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    /// Alias name of the node
    pub alias: String,
}

impl GetCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        local_cmd(run_impl(options, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: GetCommand) -> miette::Result<()> {
    let node_state = opts.state.nodes.get(cmd.alias)?;
    let addr = &node_state.config().setup().api_transport()?.addr;
    println!("Address: {addr}");
    Ok(())
}
