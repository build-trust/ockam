use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::nodes::service::ApiTransport;
use minicbor::{Decode, Encode};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Error, Result};
use ockam_multiaddr::proto::Worker;
use ockam_multiaddr::MultiAddr;
use std::net::SocketAddrV4;

/// Response body when interacting with a transport
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct TransportStatus {
    /// The type of transport to create
    #[n(1)] pub tt: TransportType,
    /// The mode the transport should operate in
    #[n(2)] pub tm: TransportMode,
    /// Corresponding socket address
    #[n(3)] pub socket_addr: String,
    /// Corresponding worker address
    #[n(4)] pub worker_addr: String,
    /// Corresponding worker address
    #[n(5)] pub processor_address: String,
    /// Corresponding flow control id
    #[n(6)] pub flow_control_id: FlowControlId,
}

impl TransportStatus {
    pub fn new(api_transport: ApiTransport) -> Self {
        Self {
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

    pub fn multiaddr(&self) -> Result<MultiAddr> {
        let mut m = MultiAddr::default();
        let worker_address = self
            .worker_addr
            .strip_prefix("0#")
            .unwrap_or(self.worker_addr.as_ref());
        m.push_back(Worker::new(worker_address))?;

        Ok(m)
    }
}

/// Response body when interacting with a transport
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct TransportList {
    #[n(1)] pub list: Vec<TransportStatus>
}

impl TransportList {
    pub fn new(list: Vec<TransportStatus>) -> Self {
        Self { list }
    }
}
