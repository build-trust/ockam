use std::array::IntoIter;

use ockam_core::lib::HashMap;

use crate::TransportError;

#[test]
fn code_and_domain() {
    let tr_errors_map = IntoIter::new([
        (0, TransportError::SendBadMessage),
        (1, TransportError::RecvBadMessage),
        (2, TransportError::BindFailed),
        (3, TransportError::ConnectionDrop),
        (4, TransportError::AlreadyConnected),
        (5, TransportError::PeerNotFound),
        (6, TransportError::PeerBusy),
        (7, TransportError::UnknownRoute),
        (8, TransportError::InvalidAddress),
        (9, TransportError::Capacity),
        (10, TransportError::Encoding),
        (11, TransportError::Protocol),
        (12, TransportError::GenericIo),
    ])
    .collect::<HashMap<_, _>>();
    for (expected_code, tr_err) in tr_errors_map {
        let err: ockam_core::Error = tr_err.into();
        assert_eq!(err.domain(), TransportError::DOMAIN_NAME);
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
    assert_eq!(err.domain(), TransportError::DOMAIN_NAME);
}

#[test]
fn from_mapped_io_errors() {
    let mapped_io_err_kinds = IntoIter::new([(
        std::io::ErrorKind::ConnectionRefused,
        TransportError::PeerNotFound,
    )])
    .collect::<HashMap<_, _>>();
    for (io_err_kind, expected_tr_err) in mapped_io_err_kinds {
        let io_err = std::io::Error::new(io_err_kind, "io::Error");
        let tr_err: TransportError = io_err.into();
        assert_eq!(tr_err, expected_tr_err);
        let err: ockam_core::Error = tr_err.into();
        assert_eq!(
            err.code(),
            TransportError::DOMAIN_CODE + expected_tr_err as u32
        );
        assert_eq!(err.domain(), TransportError::DOMAIN_NAME);
    }
}
