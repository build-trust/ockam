use core::fmt::{Display, Formatter};

/// A Bluetooth Low Energy connection worker specific error type
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum BleError {
    PermissionDenied,
    /// Functionality is not supported for this platform
    NotSupported,
    /// Failed to initialize or communicate with ble hardware
    HardwareError,
    NotFound,
    TimedOut,
    NotConnected,
    /// Device configuration failed
    ConfigurationFailed,
    /// Device failed to advertise itself
    AdvertisingFailure,
    ConnectionClosed,
    ReadError,
    WriteError,
    Other,
    Unknown,
}

impl BleError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 22_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_TRANSPORT_BLE";

    pub fn code(&self) -> u32 {
        match self {
            BleError::PermissionDenied => 0,
            BleError::NotSupported => 1,
            BleError::HardwareError => 2,
            BleError::NotFound => 3,
            BleError::TimedOut => 4,
            BleError::NotConnected => 5,
            BleError::ConfigurationFailed => 6,
            BleError::AdvertisingFailure => 7,
            BleError::ConnectionClosed => 8,
            BleError::ReadError => 9,
            BleError::WriteError => 10,
            BleError::Other => 11,
            BleError::Unknown => 12,
        }
    }
}

impl Display for BleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let err: ockam_core::Error = (*self).into();
        err.fmt(f)
    }
}

impl From<BleError> for ockam_core::Error {
    fn from(e: BleError) -> ockam_core::Error {
        ockam_core::Error::new(BleError::DOMAIN_CODE + e.code(), BleError::DOMAIN_NAME)
    }
}

#[test]
fn code_and_domain() {
    use core::array::IntoIter;
    use ockam_core::compat::collections::HashMap;

    let ble_errors_map = IntoIter::new([
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
    .collect::<HashMap<_, _>>();
    for (expected_code, ble_err) in ble_errors_map {
        let err: ockam_core::Error = ble_err.into();
        assert_eq!(err.domain(), BleError::DOMAIN_NAME);
        assert_eq!(err.code(), BleError::DOMAIN_CODE + expected_code);
    }
}
