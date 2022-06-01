use crate::config::OckamConfig;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Name of the node.
    pub node_name: String,
}

impl DeleteCommand {
    pub fn run(_cfg: &mut OckamConfig, command: DeleteCommand) {
        println!("deleting {:?}", command)
    }
}
