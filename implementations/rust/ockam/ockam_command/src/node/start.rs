use crate::{
    help,
    node::HELP_DETAIL,
    util::{exitcode, startup},
    CommandGlobalOpts,
};
use clap::Args;
use nix::unistd::Pid;
use rand::prelude::random;

/// Start Nodes
#[derive(Clone, Debug, Args)]
#[clap(arg_required_else_help = true, help_template = help::template(HELP_DETAIL))]
pub struct StartCommand {
    /// Name of the node.
    #[clap(default_value_t = hex::encode(&random::<[u8;4]>()))]
    node_name: String,
}

impl StartCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = options.config;

        // First we check whether a PID was registered and if it is still alive.
        if let Ok(Some(pid)) = cfg.get_node_pid(&self.node_name) {
            let res = nix::sys::signal::kill(Pid::from_raw(pid), None);

            if res.is_ok() {
                eprintln!(
                    "Node '{}' already appears to be running as PID {}",
                    self.node_name, pid
                );
                std::process::exit(exitcode::IOERR);
            }
        }

        // Load the node's launch configuration
        let start_cfg = match cfg.get_startup_cfg(&self.node_name) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "failed to load startup configuration for node '{}' because: {}",
                    self.node_name, e
                );
                std::process::exit(exitcode::IOERR);
            }
        };

        println!("Attempting to restart node '{}'", self.node_name);

        // Finally run the stack of configuration commands for this node
        startup::start(&self.node_name, &cfg, &start_cfg);
    }
}
