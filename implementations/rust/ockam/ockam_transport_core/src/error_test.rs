use core::array::IntoIter;
use ockam_core::compat::collections::HashMap;

use crate::TransportError;

#[test]
fn code_and_domain() {
    let tr_errors_map = IntoIter::new([
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
