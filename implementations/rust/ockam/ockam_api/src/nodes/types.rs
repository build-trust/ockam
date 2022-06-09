//! Nodemanager API types

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;
use std::fmt::{self, Display};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

///////////////////-!  REQUEST BODIES

/// Request body when instructing a node to create a transport
#[derive(Debug, Clone, Decode, Encode)]
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
#[derive(Copy, Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
pub enum TransportType {
    /// Ockam TCP transport
    #[n(0)] Tcp,
    /// Embedded BLE transport
    #[n(1)] Ble,
    /// Websocket transport
    #[n(2)] WebSocket,
}

impl Display for TransportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Tcp => "TCP",
                Self::Ble => "BLE",
                Self::WebSocket => "Websocket",
            }
        )
    }
}

/// Encode which type of transport is being requested
#[derive(Copy, Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
pub enum TransportMode {
    /// Listen on a set address
    #[n(0)] Listen,
    /// Connect to a remote peer
    #[n(1)] Connect,
}

impl Display for TransportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Listen => "Listening",
                Self::Connect => "Remote connection",
            }
        )
    }
}

///////////////////-!  RESPONSE BODIES

/// Response body for a node status
#[derive(Debug, Clone, Decode, Encode)]
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
    #[n(5)] pub transports: u32,
}

impl<'a> NodeStatus<'a> {
    pub fn new(node_name: &'a str, status: &str, workers: u32, pid: i32, transports: u32) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            node_name: Cow::Borrowed(node_name),
            status: Cow::Owned(status.into()),
            workers,
            pid,
            transports,
        }
    }
}

/// Respons body when interacting with a transport
#[derive(Debug, Clone, Decode, Encode)]
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

impl<'a> TransportStatus<'a> {
    pub fn new(tt: TransportType, tm: TransportMode, addr: &String) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tt,
            tm,
            addr: addr.clone().into(),
        }
    }
}

/// Respons body when interacting with a transport
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct TransportList<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<5212817>,
    /// The type of transport to create
    #[n(1)] pub list: Vec<TransportStatus<'a>>
}

impl<'a> TransportList<'a> {
    pub fn new(list: Vec<TransportStatus<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}
