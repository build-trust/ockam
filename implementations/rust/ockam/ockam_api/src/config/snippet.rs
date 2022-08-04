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
#[serde(tag = "type")]
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
    Portal {
        mode: PortalMode,
        protocol: Protocol,
        /// Socket or address to bind to.  For the inlet this is a
        /// `$Protocol` socket.  For the outlet this is an Ockam
        /// MultiAddr.
        bind: String,
        /// The peer of this portal endpoint.  For an outlet this is
        /// the target remote.  For the inlet this is the outlet
        /// route.
        peer: String,
    },
    SecureChannel,
    Forwarder,
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Self::Node { .. } => "start node",
            Self::Transport { .. } => "create transport",
            Self::Portal { .. } => "create portal",
            Self::SecureChannel => "create secure-channel",
            Self::Forwarder => "create forwarder",
        })
    }
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
        f.write_str(match self {
            Self::Connector => "connector",
            Self::Receiver => "receiver",
            Self::Listener => "listener",
        })
    }
}

/// Mode of a particular portal structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PortalMode {
    Inlet,
    Outlet,
}

impl fmt::Display for PortalMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Self::Inlet => "inlet",
            Self::Outlet => "outlet",
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Protocol {
    Tcp,
}
