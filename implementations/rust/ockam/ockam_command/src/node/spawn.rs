use clap::Args;
use std::process::Command;

#[derive(Clone, Debug, Args)]
pub struct SpawnCommand {
    /// Name of the node.
    pub node_name: String,
}

impl SpawnCommand {
    pub fn run(command: SpawnCommand) {
        Command::new("ockam")
            .args(["node", "start", &command.node_name])
            .spawn()
            .expect("could not spawn node");
    }
}
