use crate::CommandGlobalOpts;
use clap::Args;
use ockam_api::cli_state::StateDirTrait;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(options, self) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, _cmd: ListCommand) -> crate::Result<()> {
    for node in opts.state.nodes.list()? {
        opts.terminal.write(&format!("Node: {}\n", node.name()))?;
    }
    Ok(())
}
