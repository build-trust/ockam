use crate::config::OckamConfig;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the node.
    pub node_name: String,
}

impl CreateCommand {
    pub fn run(cfg: &mut OckamConfig, command: CreateCommand) {
        todo!()
    }
}
