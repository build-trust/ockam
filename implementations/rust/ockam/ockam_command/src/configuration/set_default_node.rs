use crate::{
    util::{exitcode, get_final_element},
    CommandGlobalOpts,
};
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct SetDefaultNodeCommand {
    /// Name of the Node
    pub name: String,
}

impl SetDefaultNodeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let name = get_final_element(&self.name);
        if options.config.get_node(name).is_ok() {
            options.config.set_default_node(&name.to_string());
            if let Err(e) = options.config.persist_config_updates() {
                eprintln!("failed to update configuration: {}", e);
                std::process::exit(exitcode::IOERR);
            }
        } else {
            eprintln!("Node ({}) is not registered yet", self.name);
            std::process::exit(exitcode::CANTCREAT);
        }
    }
}
