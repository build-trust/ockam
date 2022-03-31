use ockam_core::{
    compat::io,
    errcode::{ErrorCode, Kind, Origin},
    thiserror, Error2,
};

/// A Transport worker specific error type
#[derive(Clone, Copy, Debug, PartialEq, thiserror::Error)]
pub enum TransportError {
    /// Failed to send a malformed message
    #[error("failed to send a malformed message")]
    SendBadMessage = 1,
    /// Failed to receive a malformed message
    #[error("failed to receive a malformed message")]
    RecvBadMessage,
    /// Failed to bind to the desired socket
    #[error("failed to bind to the desired socket")]
    BindFailed,
    /// Connection was dropped unexpectedly
    #[error("connection was dropped unexpectedly")]
    ConnectionDrop,
    /// Connection was already established
    #[error("already connected")]
    AlreadyConnected,
    /// Connection peer was not found
    #[error("connection peer was not found")]
    PeerNotFound,
    /// Peer requested the incoming connection
    #[error("connection peer is busy")]
    PeerBusy,
    /// Failed to route to an unknown recipient
    #[error("message routing failed (unknown recipient)")]
    UnknownRoute,
    /// Failed to parse the socket address
    #[error("failed to parse the socket address")]
    InvalidAddress,
    /// Failed to read message (buffer exhausted) or failed to send it (size is too big)
    #[error("failed to read message (buffer exhausted)")]
    Capacity,
    /// Failed to encode message
    // FIXME: replace with ockam_core::encoding error type
    #[error("failed to encode message")]
    Encoding,
    /// Transport protocol violation
    #[error("violation in transport protocol")]
    Protocol,
    /// A generic I/O failure
    #[error("generic I/O failure")]
    GenericIo,
    /// PortalInvalidState
    #[error("portal entered invalid state")]
    PortalInvalidState,
    /// InvalidRouterResponseType
    #[error("router responded with invalid type")]
    InvalidRouterResponseType,
}

impl From<TransportError> for Error2 {
    fn from(err: TransportError) -> Error2 {
        use TransportError::*;
        let kind = match err {
            SendBadMessage => Kind::Serialization,
            RecvBadMessage => Kind::Serialization,
            BindFailed => Kind::Io,
            ConnectionDrop => Kind::Io,
            AlreadyConnected => Kind::Io,
            PeerNotFound => Kind::Misuse,
            PeerBusy => Kind::Io,
            UnknownRoute => Kind::Misuse,
            InvalidAddress => Kind::Misuse,
            Capacity => Kind::ResourceExhausted,
            Encoding => Kind::Serialization,
            Protocol => Kind::Protocol,
            GenericIo => Kind::Io,
            PortalInvalidState => Kind::Invalid,
            InvalidRouterResponseType => Kind::Invalid,
        };

        Error2::new(ErrorCode::new(Origin::Transport, kind), err)
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

// #[cfg(test)]
// mod test {
//     use ockam_core::compat::collections::HashMap;

//     use crate::TransportError;
//     #[test]
//     fn code_and_domain() {
//         let tr_errors_map = [
//             (1, TransportError::SendBadMessage),
//             (2, TransportError::RecvBadMessage),
//             (3, TransportError::BindFailed),
//             (4, TransportError::ConnectionDrop),
//             (5, TransportError::AlreadyConnected),
//             (6, TransportError::PeerNotFound),
//             (7, TransportError::PeerBusy),
//             (8, TransportError::UnknownRoute),
//             (9, TransportError::InvalidAddress),
//             (10, TransportError::Capacity),
//             (11, TransportError::Encoding),
//             (12, TransportError::Protocol),
//             (13, TransportError::GenericIo),
//             (14, TransportError::PortalInvalidState),
//         ]
//         .into_iter()
//         .collect::<HashMap<_, _>>();
//         for (expected_code, tr_err) in tr_errors_map {
//             let err: ockam_core::Error = tr_err.into();
//             assert_eq!(err.domain(), TransportError::DOMAIN_NAME);
//             assert_eq!(err.code(), TransportError::DOMAIN_CODE + expected_code);
//         }
//     }
//     #[test]
//     fn from_unmapped_io_error() {
//         let io_err = std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "io::Error");
//         let tr_err: TransportError = io_err.into();
//         assert_eq!(tr_err, TransportError::GenericIo);
//         let err: ockam_core::Error = tr_err.into();
//         assert_eq!(err.code(), TransportError::DOMAIN_CODE + tr_err as u32);
//         assert_eq!(err.domain(), TransportError::DOMAIN_NAME);
//     }
//     #[test]
//     fn from_mapped_io_errors() {
//         let mapped_io_err_kinds = [(
//             std::io::ErrorKind::ConnectionRefused,
//             TransportError::PeerNotFound,
//         )]
//         .into_iter()
//         .collect::<HashMap<_, _>>();
//         for (io_err_kind, expected_tr_err) in mapped_io_err_kinds {
//             let io_err = std::io::Error::new(io_err_kind, "io::Error");
//             let tr_err: TransportError = io_err.into();
//             assert_eq!(tr_err, expected_tr_err);
//             let err: ockam_core::Error = tr_err.into();
//             assert_eq!(
//                 err.code(),
//                 TransportError::DOMAIN_CODE + expected_tr_err as u32
//             );
//             assert_eq!(err.domain(), TransportError::DOMAIN_NAME);
//         }
//     }
// }
