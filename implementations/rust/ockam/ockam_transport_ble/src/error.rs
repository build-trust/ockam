use ockam_core::{
    errcode::{ErrorCode, Kind, Origin},
    thiserror, Error,
};

/// A Bluetooth Low Energy connection worker specific error type
#[derive(Clone, Copy, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BleError {
    #[error("permission denied")]
    PermissionDenied,
    /// Functionality is not supported for this platform
    #[error("functionality is not supported on this platform")]
    NotSupported,
    /// Failed to initialize or communicate with ble hardware
    #[error("failed to init ble hardware")]
    HardwareError,
    #[error("not found")]
    NotFound,
    #[error("timeout")]
    TimedOut,
    #[error("not connected")]
    NotConnected,
    /// Device configuration failed
    #[error("configuration failed")]
    ConfigurationFailed,
    /// Device failed to advertise itself
    #[error("ble advertising failed")]
    AdvertisingFailure,
    #[error("connection closed")]
    ConnectionClosed,
    #[error("read error")]
    ReadError,
    #[error("write error")]
    WriteError,
    #[error("other error")]
    Other,
    #[error("unknown error")]
    Unknown,
}
impl From<BleError> for Error {
    fn from(err: BleError) -> Error {
        Error::new(ErrorCode::new(Origin::Transport, Kind::Io), err)
    }
}

#[test]
#[ignore]
fn code_and_domain() {
    use ockam_core::compat::collections::HashMap;

    let ble_errors_map = IntoIterator::into_iter([
        (000, BleError::PermissionDenied),
        (001, BleError::NotSupported),
        (002, BleError::HardwareError),
        (003, BleError::NotFound),
        (004, BleError::TimedOut),
        (005, BleError::NotConnected),
        (006, BleError::ConfigurationFailed),
        (007, BleError::AdvertisingFailure),
        (008, BleError::ConnectionClosed),
        (009, BleError::ReadError),
        (010, BleError::WriteError),
        (011, BleError::Other),
        (012, BleError::Unknown),
    ])
    .collect::<HashMap<u32, BleError>>();
    for (_expected_code, ble_err) in ble_errors_map {
        let _err: Error = ble_err.into();
        // assert_eq!(err.domain(), BleError::DOMAIN_NAME);
        // assert_eq!(err.code(), BleError::DOMAIN_CODE + expected_code);
    }
}
