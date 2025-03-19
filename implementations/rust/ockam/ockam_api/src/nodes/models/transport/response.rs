use crate::colors::color_primary;
use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::nodes::service::ApiTransport;
use crate::output::Output;
use crate::terminal::fmt;
use minicbor::{CborLen, Decode, Encode};
use ockam::tcp::{TcpConnection, TcpListener, TcpListenerInfo, TcpSenderInfo};
use ockam::Message;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{cbor_encode_preallocate, Decodable, Encodable, Encoded, Error, Result};
use ockam_multiaddr::proto::Worker;
use ockam_multiaddr::MultiAddr;
use std::fmt::{Display, Formatter};
use std::net::SocketAddrV4;

/// Response body when interacting with a transport
#[derive(Debug, Clone, Encode, Decode, CborLen, serde::Serialize, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct TransportStatus {
    /// The type of transport to create
    #[serde(rename = "type")]
    #[n(1)] pub tt: TransportType,
    /// The mode the transport should operate in
    #[serde(rename = "mode")]
    #[n(2)] pub tm: TransportMode,
    /// Corresponding socket address
    #[n(3)] pub socket_address: String,
    /// Corresponding worker address
    #[n(4)] pub worker_address: String,
    /// Corresponding worker address
    #[n(5)] pub processor_address: String,
    /// Corresponding flow control id
    #[n(6)] pub flow_control_id: FlowControlId,
}

impl Encodable for TransportStatus {
    fn encode(self) -> Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for TransportStatus {
    fn decode(e: &[u8]) -> Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl TransportStatus {
    pub fn new(api_transport: ApiTransport) -> Self {
        api_transport.into()
    }

    pub fn socket_addr(&self) -> Result<SocketAddrV4> {
        self.socket_address
            .parse::<SocketAddrV4>()
            .map_err(|err| Error::new(Origin::Transport, Kind::Invalid, err))
    }

    pub fn multiaddr(&self) -> Result<MultiAddr> {
        let mut m = MultiAddr::default();
        let worker_address = self
            .worker_address
            .strip_prefix("0#")
            .unwrap_or(self.worker_address.as_ref());
        m.push_back(Worker::new(worker_address))?;

        Ok(m)
    }
}

impl From<ApiTransport> for TransportStatus {
    fn from(value: ApiTransport) -> Self {
        Self {
            tt: value.tt,
            tm: value.tm,
            socket_address: value.socket_address.to_string(),
            worker_address: value.worker_address.clone(),
            processor_address: value.processor_address.clone(),
            flow_control_id: value.flow_control_id,
        }
    }
}

impl From<TcpSenderInfo> for TransportStatus {
    fn from(value: TcpSenderInfo) -> Self {
        Self {
            tt: TransportType::Tcp,
            tm: (*value.mode()).into(),
            socket_address: value.socket_address().to_string(),
            worker_address: value.address().to_string(),
            processor_address: value.receiver_address().to_string(),
            flow_control_id: value.flow_control_id().clone(),
        }
    }
}

impl From<TcpListenerInfo> for TransportStatus {
    fn from(value: TcpListenerInfo) -> Self {
        Self {
            tt: TransportType::Tcp,
            tm: TransportMode::Listen,
            socket_address: value.socket_address().to_string(),
            worker_address: "<none>".into(),
            processor_address: value.address().to_string(),
            flow_control_id: value.flow_control_id().clone(),
        }
    }
}

impl From<TcpConnection> for TransportStatus {
    fn from(value: TcpConnection) -> Self {
        Self {
            tt: TransportType::Tcp,
            tm: TransportMode::Outgoing,
            socket_address: value.socket_address().to_string(),
            worker_address: value.sender_address().to_string(),
            processor_address: value.receiver_address().to_string(),
            flow_control_id: value.flow_control_id().clone(),
        }
    }
}

impl From<TcpListener> for TransportStatus {
    fn from(value: TcpListener) -> Self {
        Self {
            tt: TransportType::Tcp,
            tm: TransportMode::Listen,
            socket_address: value.socket_address().to_string(),
            worker_address: "<none>".into(),
            processor_address: value.processor_address().to_string(),
            flow_control_id: value.flow_control_id().clone(),
        }
    }
}

impl Display for TransportStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}:", color_primary(&self.worker_address))?;
        writeln!(f, "{}{} {} Connection", fmt::INDENTATION, self.tt, self.tm,)?;
        writeln!(
            f,
            "{}bound to {}",
            fmt::INDENTATION,
            color_primary(&self.socket_address)
        )?;
        Ok(())
    }
}

impl Output for TransportStatus {
    fn item(&self) -> crate::Result<String> {
        Ok(self.padded_display())
    }
}

#[derive(Encode, Decode, CborLen, Debug, Default, Clone, Message)]
#[cbor(transparent)]
pub struct TransportStatusList(#[n(0)] pub Vec<TransportStatus>);

impl Encodable for TransportStatusList {
    fn encode(self) -> Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for TransportStatusList {
    fn decode(e: &[u8]) -> Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}
