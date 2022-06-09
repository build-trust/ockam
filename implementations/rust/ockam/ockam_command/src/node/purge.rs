//! A simple command to purge existing configuration

use crate::util::OckamConfig;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct PurgeCommand {
    /// Should nodes be terminated with SIGKILL instead of SIGTERM
    #[clap(display_order = 900, long, short)]
    sigkill: bool,
}

impl PurgeCommand {
    pub fn run(cfg: &mut OckamConfig, command: PurgeCommand) {
        let nodes: Vec<_> = cfg
            .get_nodes()
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        for node_name in nodes {
            crate::node::delete::delete_node(cfg, &node_name, command.sigkill);
        }
    }
}
