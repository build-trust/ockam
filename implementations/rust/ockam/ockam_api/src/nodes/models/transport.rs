use minicbor::{Decode, Encode};
use ockam_core::{CowStr, Result};
use std::fmt::{self, Display};

use crate::cli_state::CliStateError;
use crate::config::lookup::InternetAddress;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Tcp};
use ockam_multiaddr::MultiAddr;

///////////////////-!  REQUEST BODIES

/// Request body when instructing a node to create a transport
#[derive(Debug, Clone, Decode, Encode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateTransport<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1503320>,
    /// The type of transport to create
    #[n(1)] pub tt: TransportType,
    /// The mode the transport should operate in
    #[n(2)] pub tm: TransportMode,
    /// The address payload for the transport
    #[b(3)] pub addr: CowStr<'a>,
}

impl<'a> CreateTransport<'a> {
    pub fn new<S: Into<CowStr<'a>>>(tt: TransportType, tm: TransportMode, addr: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tt,
            tm,
            addr: addr.into(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CreateTransportJson {
    pub tt: TransportType,
    /// The mode the transport should operate in
    pub tm: TransportMode,
    /// The address payload for the transport
    pub addr: InternetAddress,
}

impl CreateTransportJson {
    pub fn new(tt: TransportType, tm: TransportMode, addr: &str) -> Result<Self> {
        Ok(Self {
            tt,
            tm,
            addr: InternetAddress::new(addr).ok_or(CliStateError::Unknown)?,
        })
    }

    pub fn maddr(&self) -> Result<MultiAddr> {
        let mut m = MultiAddr::default();
        let addr = &self.addr;
        match addr {
            InternetAddress::Dns(dns, _) => m.push_back(DnsAddr::new(dns))?,
            InternetAddress::V4(v4) => m.push_back(Ip4(*v4.ip()))?,
            InternetAddress::V6(v6) => m.push_back(Ip6(*v6.ip()))?,
        }
        m.push_back(Tcp(addr.port()))?;
        Ok(m)
    }
}

/// Request to delete a transport
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteTransport<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4739996>,
    /// The transport ID to delete
    #[b(1)] pub tid: CowStr<'a>,
}

impl<'a> DeleteTransport<'a> {
    pub fn new<S: Into<CowStr<'a>>>(tid: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tid: tid.into(),
        }
    }
}

/// Encode which type of transport is being requested
// TODO: we have a TransportType in ockam_core.  Do we really want to
// mirror this kind of type here?
#[derive(Copy, Clone, Debug, Decode, Encode, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(index_only)]
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
        f.write_str(match self {
            Self::Tcp => "TCP",
            Self::Ble => "BLE",
            Self::WebSocket => "Websocket",
        })
    }
}

/// Encode which type of transport is being requested
#[derive(Copy, Clone, Debug, Decode, Encode, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[rustfmt::skip]
pub enum TransportMode {
    /// Listen on a set address
    #[n(0)] Listen,
    /// Connect to a remote peer
    #[n(1)] Connect,
}

impl Display for TransportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Listen => "Listening",
            Self::Connect => "Remote connection",
        })
    }
}

///////////////////-!  RESPONSE BODIES

/// Response body when interacting with a transport
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct TransportStatus<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1581592>,
    /// The type of transport to create
    #[n(2)] pub tt: TransportType,
    /// The mode the transport should operate in
    #[n(3)] pub tm: TransportMode,
    /// Corresponding socket address
    #[b(4)] pub socket_addr: CowStr<'a>,
    /// Corresponding worker address
    #[b(5)] pub worker_addr: CowStr<'a>,
    /// Transport ID inside the node manager
    ///
    /// We use this as a kind of URI to be able to address a transport
    /// by a unique value for specific updates and deletion events.
    #[b(6)] pub tid: CowStr<'a>,
}

impl<'a> TransportStatus<'a> {
    pub fn new(
        tt: TransportType,
        tm: TransportMode,
        socket_addr: impl Into<CowStr<'a>>,
        worker_addr: impl Into<CowStr<'a>>,
        tid: impl Into<CowStr<'a>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tt,
            tm,
            socket_addr: socket_addr.into(),
            worker_addr: worker_addr.into(),
            tid: tid.into(),
        }
    }
}

/// Response body when interacting with a transport
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct TransportList<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5212817>,
    #[b(1)] pub list: Vec<TransportStatus<'a>>
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
