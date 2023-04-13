use ockam_core::{
    compat::io,
    errcode::{Kind, Origin},
    Error,
};

/// A Transport worker specific error type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    // FIXME: replace with ockam_core::encoding error type
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

impl ockam_core::compat::error::Error for TransportError {}
impl core::fmt::Display for TransportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SendBadMessage => write!(f, "failed to send a malformed message"),
            Self::RecvBadMessage => write!(f, "failed to receive a malformed message"),
            Self::BindFailed => write!(f, "failed to bind to the desired socket"),
            Self::ConnectionDrop => write!(f, "connection was dropped unexpectedly"),
            Self::AlreadyConnected => write!(f, "already connected"),
            Self::PeerNotFound => write!(f, "connection peer was not found"),
            Self::PeerBusy => write!(f, "connection peer is busy"),
            Self::UnknownRoute => write!(f, "message routing failed (unknown recipient)"),
            Self::InvalidAddress => write!(f, "failed to parse the socket address"),
            Self::Capacity => write!(f, "failed to read message (buffer exhausted)"),
            Self::Encoding => write!(f, "failed to encode message"),
            Self::Protocol => write!(f, "violation in transport protocol"),
            Self::GenericIo => write!(f, "generic I/O failure"),
            Self::PortalInvalidState => write!(f, "portal entered invalid state"),
            Self::InvalidRouterResponseType => write!(f, "router responded with invalid type"),
        }
    }
}

impl From<TransportError> for Error {
    #[track_caller]
    fn from(err: TransportError) -> Error {
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

        Error::new(Origin::Transport, kind, err)
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
