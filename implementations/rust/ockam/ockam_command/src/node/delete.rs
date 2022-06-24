use crate::util::OckamConfig;
use clap::Args;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Name of the node to delete
    pub node_name: String,
    /// Should the node be terminated with SIGKILL instead of SIGTERM
    #[clap(display_order = 900, long, short)]
    sigkill: bool,
}

impl DeleteCommand {
    pub fn run(cfg: &OckamConfig, command: DeleteCommand) {
        delete_node(cfg, &command.node_name, command.sigkill);
    }
}

pub fn delete_node(cfg: &OckamConfig, node_name: &String, sigkill: bool) {
    let pid = match cfg.get_node_pid(node_name) {
        Ok(pid) => pid,
        Err(e) => {
            eprintln!("Failed to delete node: {}", e);
            std::process::exit(-1);
        }
    };

    if let Some(pid) = pid {
        if let Err(e) = signal::kill(
            Pid::from_raw(pid),
            if sigkill {
                Signal::SIGKILL
            } else {
                Signal::SIGTERM
            },
        ) {
            eprintln!("Error occurred while terminating node process: {}", e);
        }
    }

    if let Err(e) = cfg.delete_node(node_name) {
        eprintln!("failed to remove node from config: {}", e);
    }

    if let Err(e) = cfg.atomic_update().run() {
        eprintln!("failed to update configuration: {}", e);
    }

    eprintln!("Deleted node '{}'", node_name);
}
