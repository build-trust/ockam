use serde::{Deserialize, Serialize};
use std::fmt;

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
        api_addr: String,
        node_name: String,
    },
    Transport {
        mode: RemoteMode,
        protocol: Protocol,
        address: String,
    },
    Portal,
    SecureChannel,
    Forwarder,
}

/// The mode a remote operation is using
///
/// * A `Connector` is a connection initiator.  It can either contact a
/// `Socket` or a `Listener`
///
/// * A `Receiver` is a fully fledged, static responder, meaning it only
/// handles a connection from a single `Connector`
///
/// * A `Listener` spawns `Receiver`s for any incoming `Connector`
/// handshake
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RemoteMode {
    Connector,
    Receiver,
    Listener,
}

impl fmt::Display for RemoteMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Connector => "connector",
                Self::Receiver => "receiver",
                Self::Listener => "listener",
            }
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Protocol {
    Tcp,
}
