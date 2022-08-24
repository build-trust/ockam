use crate::{util::exitcode, CommandGlobalOpts};
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct GetDefaultNodeCommand {}

impl GetDefaultNodeCommand {
    pub fn run(opts: CommandGlobalOpts, _command: GetDefaultNodeCommand) {
        match opts.config.get_default_node() {
            Some(name) => {
                println!("Current Default Node: {}", name)
            }
            None => {
                eprintln!("Default Node is not set");
                std::process::exit(exitcode::UNAVAILABLE);
            }
        }
    }
}
