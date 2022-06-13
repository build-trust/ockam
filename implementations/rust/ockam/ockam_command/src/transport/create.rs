use crate::util::OckamConfig;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the node.
    pub node_name: String,
}

impl CreateCommand {
    pub fn run(_cfg: &mut OckamConfig, _command: CreateCommand) {
        todo!()
    }
}
