//! Watchdog service utility

mod service;
pub use service::Watchdog;

use super::OckamConfig;
use clap::Args;
use std::path::PathBuf;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub fn socket_path(opts: CommandGlobalOpts, node_name: &str) -> PathBuf {
    let cfg = &opts.config;
    let node_dir = cfg
        .get_node_dir(node_name)
        .expect("this shouldn't happen, is there a race condition? (there always is)");
    node_dir.join("_watchdog.socket")
}

#[derive(Clone, Debug, Args)]
pub struct WatchdogCommand {
    node_name: String,
}

impl WatchdogCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: WatchdogCommand) {
        let cfg = &opts.config;
        let socket_path = socket_path(cfg, &cmd.node_name);

        Watchdog { socket_path }.run()

        // let stream = listener.incoming().next();
    }
}

// enum Instruction {
//     /// Restart this node
//     Restart,
//     /// Stop the node and prevent it from restarting
//     StopAndHalt,
// }

// struct NodeSide(Receiver<Instruction>);

// struct UserSide(Sender<Instruction>);
