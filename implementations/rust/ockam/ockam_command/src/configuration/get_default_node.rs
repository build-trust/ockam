use anyhow::anyhow;
use clap::Args;

use crate::exitcode::UNAVAILABLE;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct GetDefaultNodeCommand {}

impl GetDefaultNodeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(options) {
            eprintln!("{}", e);
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts) -> crate::Result<()> {
    match opts.config.get_default_node() {
        Some(name) => {
            println!("Current Default Node: {}", name);
            Ok(())
        }
        None => Err(crate::error::Error::new(
            UNAVAILABLE,
            anyhow!("Default Node is not set"),
        )),
    }
}
