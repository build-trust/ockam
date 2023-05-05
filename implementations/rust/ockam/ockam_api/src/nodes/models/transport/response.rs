use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::nodes::service::ApiTransport;
use minicbor::{Decode, Encode};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{CowStr, Error, Result};
use std::net::SocketAddrV4;

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
    /// Corresponding flow control id
    #[n(6)] pub flow_control_id: FlowControlId,
    /// Transport ID inside the node manager
    ///
    /// We use this as a kind of URI to be able to address a transport
    /// by a unique value for specific updates and deletion events.
    #[b(7)] pub tid: CowStr<'a>,
}

impl<'a> TransportStatus<'a> {
    pub fn new(api_transport: ApiTransport, tid: impl Into<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tt: api_transport.tt,
            tm: api_transport.tm,
            socket_addr: CowStr::from(api_transport.socket_address.to_string()),
            worker_addr: CowStr::from(api_transport.worker_address.to_string()),
            flow_control_id: api_transport.flow_control_id,
            tid: tid.into(),
        }
    }

    pub fn socket_addr(&self) -> Result<SocketAddrV4> {
        self.socket_addr
            .parse::<SocketAddrV4>()
            .map_err(|err| Error::new(Origin::Transport, Kind::Invalid, err))
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
