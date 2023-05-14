use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::nodes::service::ApiTransport;
use minicbor::{Decode, Encode};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{Error, Result};
use std::net::SocketAddrV4;

/// Response body when interacting with a transport
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct TransportStatus {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1581592>,
    /// The type of transport to create
    #[n(2)] pub tt: TransportType,
    /// The mode the transport should operate in
    #[n(3)] pub tm: TransportMode,
    /// Corresponding socket address
    #[n(4)] pub socket_addr: String,
    /// Corresponding worker address
    #[n(5)] pub worker_addr: String,
    /// Corresponding worker address
    #[n(6)] pub processor_address: String,
    /// Corresponding flow control id
    #[n(7)] pub flow_control_id: FlowControlId,
}

impl TransportStatus {
    pub fn new(api_transport: ApiTransport) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tt: api_transport.tt,
            tm: api_transport.tm,
            socket_addr: api_transport.socket_address.to_string(),
            worker_addr: api_transport.worker_address.clone(),
            processor_address: api_transport.processor_address.clone(),
            flow_control_id: api_transport.flow_control_id,
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
pub struct TransportList {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5212817>,
    #[n(1)] pub list: Vec<TransportStatus>
}

impl TransportList {
    pub fn new(list: Vec<TransportStatus>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}
