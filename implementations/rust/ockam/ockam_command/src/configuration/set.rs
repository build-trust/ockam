use anyhow::anyhow;

use crate::{util::exitcode, CommandGlobalOpts, Error};
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
        if let Err(e) = run_impl(options, self) {
            eprintln!("{}", e);
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: SetCommand) -> crate::Result<()> {
    let target_addr = InternetAddress::new(&cmd.target).ok_or_else(|| {
        let message = anyhow!("Invalid alias address! Please provide an address in the following schema: <address>:<port>. \
                     IPv6, IPv4, and DNS addresses are supported!");
        Error::new(exitcode::USAGE, message)
    })?;

    opts.config.set_node_alias(cmd.name, target_addr);
    opts.config.persist_config_updates().map_err(|e| {
        let message = format!("Failed to persist config updates -- {}", e);
        Error::new(exitcode::IOERR, anyhow!(message))
    })
}
