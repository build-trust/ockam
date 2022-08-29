use crate::{
    help,
    node::HELP_DETAIL,
    util::{exitcode, startup},
    CommandGlobalOpts,
};
use clap::Args;
use rand::prelude::random;

/// Stop Nodes
#[derive(Clone, Debug, Args)]
#[clap(help_template = help::template(HELP_DETAIL))]
pub struct StopCommand {
    /// Name of the node.
    #[clap(default_value_t = hex::encode(&random::<[u8;4]>()))]
    node_name: String,
    /// Whether to use the SIGTERM or SIGKILL signal to stop the node
    #[clap(long)]
    force: bool,
}

impl StopCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = options.config;
        match cfg.get_node_pid(&self.node_name) {
            Ok(Some(pid)) => {
                if let Err(e) = startup::stop(pid, self.force) {
                    eprintln!("{e:?}");
                    std::process::exit(exitcode::OSERR);
                }
            }
            Ok(_) => {
                eprintln!("Node {} is not running!", &self.node_name);
                std::process::exit(exitcode::IOERR);
            }
            Err(_) => {
                eprintln!("Node {} does not exist!", &self.node_name);
                std::process::exit(exitcode::IOERR);
            }
        };
    }
}
