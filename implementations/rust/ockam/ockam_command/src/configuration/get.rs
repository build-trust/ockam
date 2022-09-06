use crate::{util::exitcode, CommandGlobalOpts};
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    /// Alias name of the node
    pub alias: String,
}

impl GetCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let lookup = options.config.lookup();
        match lookup.get_node(&self.alias) {
            Some(addr) => {
                println!("Node: {}\nAddress: {}", self.alias, addr);
            }
            None => {
                eprintln!(
                    "Alias {} not known.  Add it first with `ockam alias set`!",
                    self.alias
                );
                std::process::exit(exitcode::DATAERR);
            }
        }
    }
}
