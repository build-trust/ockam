//! A simple command to purge existing configuration

use crate::CommandGlobalOpts;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct PurgeCommand {
    /// Should nodes be terminated with SIGKILL instead of SIGTERM
    #[clap(display_order = 900, long, short)]
    sigkill: bool,
}

impl PurgeCommand {
    pub fn run(opts: CommandGlobalOpts, command: PurgeCommand) {
        let cfg = &opts.config;
        let nodes: Vec<_> = cfg
            .get_inner()
            .nodes
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        for node_name in nodes {
            crate::node::delete::delete_node(&opts, &node_name, command.sigkill);
        }
    }
}
