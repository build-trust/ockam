use ockam_core::{
    errcode::{ErrorCode, Kind, Origin},
    thiserror, Error2,
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
impl From<BleError> for Error2 {
    fn from(err: BleError) -> Error2 {
        Error2::new(ErrorCode::new(Origin::Transport, Kind::Io), err)
    }
}

#[test]
fn code_and_domain() {
    let ble_errors_map = [
        (000_u32, BleError::PermissionDenied),
        (001_u32, BleError::NotSupported),
        (002_u32, BleError::HardwareError),
        (003_u32, BleError::NotFound),
        (004_u32, BleError::TimedOut),
        (005_u32, BleError::NotConnected),
        (006_u32, BleError::ConfigurationFailed),
        (007_u32, BleError::AdvertisingFailure),
        (008_u32, BleError::ConnectionClosed),
        (009_u32, BleError::ReadError),
        (010_u32, BleError::WriteError),
        (011_u32, BleError::Other),
        (012_u32, BleError::Unknown),
    ];
    for (expected_code, ble_err) in ble_errors_map {
        let err: ockam_core::Error = ble_err.into();
        assert_eq!(err.code(), BleError::DOMAIN_CODE + expected_code);
    }
}
