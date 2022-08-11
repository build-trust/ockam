use crate::{
    util::{exitcode, startup},
    CommandGlobalOpts,
};
use clap::Args;
use nix::unistd::Pid;
use rand::prelude::random;

#[derive(Clone, Debug, Args)]
pub struct StartCommand {
    /// Name of the node.
    #[clap(default_value_t = hex::encode(&random::<[u8;4]>()))]
    node_name: String,
}

impl StartCommand {
    pub fn run(opts: CommandGlobalOpts, command: Self) {
        let cfg = opts.config;

        // First we check whether a PID was registered and if it is still alive.
        if let Ok(Some(pid)) = cfg.get_node_pid(&command.node_name) {
            let res = nix::sys::signal::kill(Pid::from_raw(pid), None);

            if res.is_ok() {
                eprintln!(
                    "Node '{}' already appears to be running as PID {}",
                    command.node_name, pid
                );
                std::process::exit(exitcode::IOERR);
            }
        }

        // Load the node's launch configuration
        let start_cfg = match cfg.get_startup_cfg(&command.node_name) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "failed to load startup configuration for node '{}' because: {}",
                    command.node_name, e
                );
                std::process::exit(exitcode::IOERR);
            }
        };

        println!("Attempting to restart node '{}'", command.node_name);

        // Finally run the stack of configuration commands for this node
        startup::start(&command.node_name, &cfg, &start_cfg);
    }
}
