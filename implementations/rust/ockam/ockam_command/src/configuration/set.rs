use crate::CommandGlobalOpts;
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
    pub fn run(opts: CommandGlobalOpts, command: SetCommand) {
        let target_addr = match InternetAddress::new(&command.target) {
            Some(addr) => addr,
            None => {
                eprintln!(
                    "Invalid alias address!  Please provide an address in the following schema: <address>:<port>. \
                     IPv6, IPv4, and DNS addresses are supported!"
                );
                std::process::exit(-1);
            }
        };

        opts.config.set_node_alias(command.name, target_addr);
    }
}
