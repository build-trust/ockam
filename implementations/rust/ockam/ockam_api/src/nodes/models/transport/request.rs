use minicbor::{CborLen, Decode, Encode};

/// Request body when instructing a node to create a transport
#[derive(Debug, Clone, Encode, Decode, CborLen, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateTcpConnection {
    /// The address payload for the transport
    #[n(1)] pub addr: String,
}

impl CreateTcpConnection {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }
}

/// Request body when instructing a node to create a transport
#[derive(Debug, Clone, Encode, Decode, CborLen, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateTcpListener {
    /// The address payload for the transport
    #[n(1)] pub addr: String,
}

impl CreateTcpListener {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }
}

/// Request to delete a transport
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteTransport {
    /// The transport processor address or socket address to delete
    #[n(1)] pub address: String,
}

impl DeleteTransport {
    pub fn new(address: String) -> Self {
        Self { address }
    }
}
