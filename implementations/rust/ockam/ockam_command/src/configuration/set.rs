use crate::{util::exitcode, CommandGlobalOpts};
use clap::Args;
use ockam_api::config::lookup::InternetAddress;

#[derive(Clone, Debug, Args)]
pub struct SetCommand {
    /// Name of the configuration value
    pub name: String,
    /// The payload to update the config value with
    pub target: String,
}

impl SetCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let target_addr = match InternetAddress::new(&self.target) {
            Some(addr) => addr,
            None => {
                eprintln!(
                    "Invalid alias address!  Please provide an address in the following schema: <address>:<port>. \
                     IPv6, IPv4, and DNS addresses are supported!"
                );
                std::process::exit(exitcode::USAGE);
            }
        };

        options.config.set_node_alias(self.name, target_addr);
        if let Err(e) = options.config.persist_config_updates() {
            eprintln!("{}", e);
            std::process::exit(exitcode::IOERR);
        }
    }
}
