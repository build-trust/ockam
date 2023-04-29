use clap::Args;

use crate::node::{default_node_name, node_name_parser};

#[derive(Clone, Debug, Args)]
pub struct SecureChannelListenerNodeOpts {
    /// Node
    #[arg(global = true, long, value_name = "NODE", default_value_t = default_node_name(), value_parser = node_name_parser)]
    pub at: String,
}
