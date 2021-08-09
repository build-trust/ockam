/// A Transport worker specific error type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransportError {
    /// Failed to send a malformed message
    SendBadMessage,
    /// Failed to receive a malformed message
    RecvBadMessage,
    /// Failed to bind to the desired socket
    BindFailed,
    /// Connection was dropped unexpectedly
    ConnectionDrop,
    /// Connection was already established
    AlreadyConnected,
    /// Connection peer was not found
    PeerNotFound,
    /// Peer requested the incoming connection
    PeerBusy,
    /// Failed to route to an unknown recipient
    UnknownRoute,
    /// Failed to parse the socket address
    InvalidAddress,
    /// Failed to read message (buffer exhausted) or failed to send it (size is too big)
    Capacity,
    /// Failed to encode message
    Encoding,
    /// Transport protocol violation
    Protocol,
    /// A generic I/O failure
    GenericIo,
}

impl TransportError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 15_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_TRANSPORT_CORE";
}

impl From<TransportError> for ockam_core::Error {
    fn from(e: TransportError) -> ockam_core::Error {
        ockam_core::Error::new(
            TransportError::DOMAIN_CODE + (e as u32),
            TransportError::DOMAIN_NAME,
        )
    }
}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::ConnectionRefused => Self::PeerNotFound,
            _ => Self::GenericIo,
        }
    }
}
