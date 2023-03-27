use clap::Args;

use crate::node::default_node_name;

#[derive(Clone, Debug, Args)]
pub struct SecureChannelListenerNodeOpts {
    /// Node
    #[arg(global = true, long, value_name = "NODE", default_value_t = default_node_name())]
    pub at: String,
}
