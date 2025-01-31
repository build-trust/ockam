use minicbor::{CborLen, Decode, Encode};
use ockam::Message;
use ockam_core::{cbor_encode_preallocate, Decodable, Encodable, Encoded};

/// Request body when instructing a node to create a transport
#[derive(Debug, Clone, Encode, Decode, CborLen, PartialEq, Eq, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateTcpConnection {
    /// The address payload for the transport
    #[n(1)] pub addr: String,
}

impl Encodable for CreateTcpConnection {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for CreateTcpConnection {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl CreateTcpConnection {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }
}

/// Request body when instructing a node to create a transport
#[derive(Debug, Clone, Encode, Decode, CborLen, PartialEq, Eq, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateTcpListener {
    /// The address payload for the transport
    #[n(1)] pub addr: String,
}

impl Encodable for CreateTcpListener {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for CreateTcpListener {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl CreateTcpListener {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }
}

/// Request to delete a transport
#[derive(Debug, Clone, Encode, Decode, CborLen, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteTransport {
    /// The transport processor address or socket address to delete
    #[n(1)] pub address: String,
}

impl Encodable for DeleteTransport {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for DeleteTransport {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl DeleteTransport {
    pub fn new(address: String) -> Self {
        Self { address }
    }
}
