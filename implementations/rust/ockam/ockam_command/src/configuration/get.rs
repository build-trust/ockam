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
        if let Err(e) = run_impl(options, self) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: GetCommand) -> crate::Result<()> {
    let node_state = opts.state.nodes.get(cmd.alias)?;
    let addr = &node_state.config().setup().default_tcp_listener()?.addr;
    println!("Address: {addr}");
    Ok(())
}
