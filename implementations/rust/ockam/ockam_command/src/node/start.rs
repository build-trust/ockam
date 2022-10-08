use crate::{
    help,
    node::HELP_DETAIL,
    util::{exitcode, startup::spawn_node},
    CommandGlobalOpts,
};
use clap::Args;
use nix::unistd::Pid;
use rand::prelude::random;

/// Start Nodes
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, help_template = help::template(HELP_DETAIL))]
pub struct StartCommand {
    /// Name of the node.
    #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    node_name: String,
}

impl StartCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        let cfg = &opts.config;
        let cfg_node = cfg
            .get_node(&self.node_name)
            .expect("failed to load node config");

        // First we check whether a PID was registered and if it is still alive.
        if let Some(pid) = cfg_node.pid {
            // Note: On CI machines where <defunct> processes can occur,
            // the below `kill 0 pid` can imply a killed process is okay.
            let res = nix::sys::signal::kill(Pid::from_raw(pid), None);

            if res.is_ok() {
                eprintln!(
                    "Node '{}' already appears to be running as PID {}",
                    self.node_name, pid
                );
                std::process::exit(exitcode::IOERR);
            }
        }

        // Construct the arguments list and re-execute the ockam
        // CLI in foreground mode to re-start the node
        spawn_node(
            &opts.config,               // Ockam configuration
            cfg_node.verbose,           // Previously user-chosen verbosity level
            true,                       // skip-defaults because the node already exists
            false,                      // Default value. TODO: implement persistence of this option
            false,                      // Default value. TODO: implement persistence of this option
            &cfg_node.name,             // The selected node name
            &cfg_node.addr.to_string(), // The selected node api address
            None,                       // No project information available
        );
    }
}
