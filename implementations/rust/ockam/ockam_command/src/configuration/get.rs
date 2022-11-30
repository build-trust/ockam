use crate::CommandGlobalOpts;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    /// Alias name of the node
    pub alias: String,
}

impl GetCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(options, self) {
            eprintln!("{}", e);
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: GetCommand) -> crate::Result<()> {
    let node_setup = opts.state.nodes.get(&cmd.alias)?.setup()?;
    let addr = &node_setup.default_tcp_listener()?.addr;
    println!("Address: {addr}");
    Ok(())
}
