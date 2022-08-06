use crate::{CommandGlobalOpts, OckamConfig};
use clap::Args;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::ops::Deref;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Name of the node.
    #[clap(default_value = "default")]
    node_name: String,

    /// Terminate all nodes
    #[clap(long, short)]
    all: bool,

    /// Should the node be terminated with SIGKILL instead of SIGTERM
    #[clap(display_order = 900, long, short)]
    sigkill: bool,

    /// Clean up config directories and all nodes state directories
    #[clap(display_order = 901, long, short)]
    force: bool,
}

impl DeleteCommand {
    pub fn run(opts: CommandGlobalOpts, command: DeleteCommand) {
        if command.all {
            let cfg = &opts.config;
            let node_names: Vec<String> = {
                let inner = cfg.get_inner();

                if inner.nodes.is_empty() {
                    eprintln!("No nodes registered on this system!");
                    std::process::exit(0);
                }

                inner.nodes.iter().map(|(name, _)| name.clone()).collect()
            };

            for node in node_names {
                delete_node(&opts, &node, command.sigkill)
            }

            if command.force {
                remove_config_dir(cfg);
            }
        } else {
            delete_node(&opts, &command.node_name, command.sigkill);
        }
    }
}

pub fn delete_node(opts: &CommandGlobalOpts, node_name: &String, sigkill: bool) {
    let cfg = &opts.config;
    let pid = match cfg.get_node_pid(node_name) {
        Ok(pid) => pid,
        Err(e) => {
            eprintln!("Failed to delete node: {}", e);
            std::process::exit(-1);
        }
    };

    if let Some(pid) = pid {
        let _ = signal::kill(
            Pid::from_raw(pid),
            if sigkill {
                Signal::SIGKILL
            } else {
                Signal::SIGTERM
            },
        );
    }

    if let Err(e) = cfg.get_node_dir(node_name).map(std::fs::remove_dir_all) {
        eprintln!("Failed to delete node directory: {}", e);
    }

    if let Err(e) = cfg.delete_node(node_name) {
        eprintln!("failed to remove node from config: {}", e);
    }

    if let Err(e) = cfg.atomic_update().run() {
        eprintln!("failed to update configuration: {}", e);
    }

    eprintln!("Deleted node '{}'", node_name);
}

fn remove_config_dir(cfg: &OckamConfig) {
    let inner = cfg.deref().writelock_inner();
    let config_dir = inner
        .directories
        .as_ref()
        .expect("configuration is in an invalid state")
        .config_dir();

    if let Err(e) = std::fs::remove_dir_all(config_dir) {
        eprintln!("Failed to delete config directory: {}", e);
    }
}
