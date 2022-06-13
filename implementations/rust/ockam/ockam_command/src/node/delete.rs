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
    pub fn run(cfg: &mut OckamConfig, command: DeleteCommand) {
        delete_node(cfg, &command.node_name, command.sigkill);
    }
}

pub fn delete_node(cfg: &mut OckamConfig, node_name: &String, sigkill: bool) {
    let node_cfg = match cfg.get_nodes().get(node_name) {
        Some(node_cfg) => node_cfg,
        None => {
            eprintln!("No such node registired");
            std::process::exit(-1);
        }
    };

    if let Some(pid) = node_cfg.pid {
        if let Err(e) = signal::kill(
            Pid::from_raw(pid),
            if sigkill {
                Signal::SIGKILL
            } else {
                Signal::SIGTERM
            },
        ) {
            eprintln!("Error occured while terminating node process: {}", e);
        }
    }

    if let Err(e) = cfg.delete_node(node_name) {
        eprintln!("failed to remove node from config: {}", e);
    }

    cfg.save();
    println!("Deleted node '{}'", node_name);
}
