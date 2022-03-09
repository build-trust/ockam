use ockam_core::compat::io;

/// A Transport worker specific error type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransportError {
    /// Failed to send a malformed message
    SendBadMessage = 1,
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
    /// PortalInvalidState
    PortalInvalidState,
    /// InvalidRouterResponseType
    InvalidRouterResponseType,
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
            format!("{}::{:?}", module_path!(), e),
        )
    }
}

impl From<io::Error> for TransportError {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::ConnectionRefused => Self::PeerNotFound,
            _ => Self::GenericIo,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::TransportError;

    #[test]
    fn code_and_domain() {
        let tr_errors_map = [
            (1, TransportError::SendBadMessage),
            (2, TransportError::RecvBadMessage),
            (3, TransportError::BindFailed),
            (4, TransportError::ConnectionDrop),
            (5, TransportError::AlreadyConnected),
            (6, TransportError::PeerNotFound),
            (7, TransportError::PeerBusy),
            (8, TransportError::UnknownRoute),
            (9, TransportError::InvalidAddress),
            (10, TransportError::Capacity),
            (11, TransportError::Encoding),
            (12, TransportError::Protocol),
            (13, TransportError::GenericIo),
            (14, TransportError::PortalInvalidState),
        ]
        .into_iter();
        for (expected_code, tr_err) in tr_errors_map {
            let err: ockam_core::Error = tr_err.into();
            assert_eq!(err.code(), TransportError::DOMAIN_CODE + expected_code);
        }
    }

    #[test]
    fn from_unmapped_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "io::Error");
        let tr_err: TransportError = io_err.into();
        assert_eq!(tr_err, TransportError::GenericIo);
        let err: ockam_core::Error = tr_err.into();
        assert_eq!(err.code(), TransportError::DOMAIN_CODE + tr_err as u32);
    }

    #[test]
    fn from_mapped_io_errors() {
        let mapped_io_err_kinds = [(
            std::io::ErrorKind::ConnectionRefused,
            TransportError::PeerNotFound,
        )]
        .into_iter();
        for (io_err_kind, expected_tr_err) in mapped_io_err_kinds {
            let io_err = std::io::Error::new(io_err_kind, "io::Error");
            let tr_err: TransportError = io_err.into();
            assert_eq!(tr_err, expected_tr_err);
            let err: ockam_core::Error = tr_err.into();
            assert_eq!(
                err.code(),
                TransportError::DOMAIN_CODE + expected_tr_err as u32
            );
        }
    }
}
