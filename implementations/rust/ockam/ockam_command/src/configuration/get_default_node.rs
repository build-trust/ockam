use clap::Args;

use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct GetDefaultNodeCommand {}

impl GetDefaultNodeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(options) {
            eprintln!("{e}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(_opts: CommandGlobalOpts) -> crate::Result<()> {
    // TODO: get from opts.state.nodes().default()
    todo!()
}
