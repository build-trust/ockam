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
        match options.config.select_node(name) {
            Some(_) => {
                options.config.set_default_node(&name.to_string());
                // Save the config update
                if let Err(e) = options.config.atomic_update().run() {
                    eprintln!("failed to update configuration: {}", e);
                    std::process::exit(exitcode::IOERR);
                }
            }
            None => {
                eprintln!("Node ({}) is not registered yet", self.name);
                std::process::exit(exitcode::CANTCREAT);
            }
        }
    }
}
