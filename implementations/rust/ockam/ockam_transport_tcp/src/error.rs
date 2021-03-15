use ockam::Error;

// TODO: we might be able to make this error type more generic and
// move it to an `ockam_transport` crate which would then aid authors
// of transport channels in their implementations.

/// A TCP connection worker specific error type
#[derive(Clone, Copy, Debug)]
pub enum TcpError {
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
    /// Peer requected the incoming connection
    PeerBusy,
}

impl TcpError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 15_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_TRANSPORT_TCP";
}

impl From<TcpError> for Error {
    fn from(e: TcpError) -> Error {
        Error::new(TcpError::DOMAIN_CODE + (e as u32), TcpError::DOMAIN_NAME)
    }
}
