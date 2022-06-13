use crate::util::OckamConfig;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    /// Name of the node.
    pub node_name: String,
}

impl ListCommand {
    pub fn run(_cfg: &mut OckamConfig, _command: ListCommand) {
        todo!()
    }
}
