//! Nodemanager API types

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

///////////////////-!  REQUEST BODIES

/// Request body when instructing a node to create a transport
#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateTransport<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<1407961>,
    /// The type of transport to create
    #[n(1)] tt: TransportType,
    /// The mode the transport should operate in
    #[n(2)] tm: TransportMode,
    /// The address payload for the transport
    #[n(3)] addr: Cow<'a, str>,
}

/// Encode which type of transport is being requested
#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
pub enum TransportType {
    /// Ockam TCP transport
    #[n(0)] Tcp,
    /// Embedded BLE transport
    #[n(1)] Ble,
    /// Websocket transport
    #[n(2)] WebSocket,
}

/// Encode which type of transport is being requested
#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
pub enum TransportMode {
    /// Listen on a set address
    #[n(0)] Listen,
    /// Connect to a remote peer
    #[n(1)] Connect,
}

///////////////////-!  RESPONSE BODIES

/// Response body for a node status
#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct NodeStatus<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<6586555>,
    #[n(1)] pub node_name: Cow<'a, str>,
    #[n(2)] pub status: Cow<'a, str>,
    #[n(3)] pub workers: u32,
    #[n(4)] pub pid: i32,
}

impl<'a> NodeStatus<'a> {
    pub fn new(node_name: &'a str, status: &str, workers: u32, pid: i32) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            node_name: Cow::Borrowed(node_name),
            status: Cow::Owned(status.into()),
            workers,
            pid,
        }
    }
}

/// Respons body when interacting with a transport
#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct TransportStatus<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<1581592>,
    /// The type of transport to create
    #[n(1)] pub tt: TransportType,
    /// The mode the transport should operate in
    #[n(2)] pub tm: TransportMode,
    /// The address payload for the transport
    #[n(3)] pub addr: Cow<'a, str>,
}
