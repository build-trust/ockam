use serde::{Deserialize, Serialize};

/// A composable snippet run against an existing node
///
/// The goal of this type structure is to keep track of configuration
/// changes applied to a node to allow users to quickly restart their
/// nodes with all associated state, instead of having to take care of
/// this in start-scripts themselves.
///
/// This system is also used by the ockam-watchdog.
///
/// This structure does not have to be able to express _all_ possible
/// values given to the CLI.  Many of the commands are either
/// destructive, or don't modify state at all.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComposableSnippet {
    pub id: String,
    pub op: Operation,
    pub params: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Operation {
    /// The node was created with a
    Node {
        port: u16,
        node_name: String,
    },
    Transport {
        listen: bool,
        tcp: bool,
        addr: String,
    },
    Portal,
    SecureChannel,
    Forwarder,
}
