use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

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

impl ockam_core::compat::error::Error for BleError {}
impl core::fmt::Display for BleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "permission denied"),
            Self::NotSupported => write!(f, "functionality is not supported on this platform"),
            Self::HardwareError => write!(f, "failed to init ble hardware"),
            Self::NotFound => write!(f, "not found"),
            Self::TimedOut => write!(f, "timeout"),
            Self::NotConnected => write!(f, "not connected"),
            Self::ConfigurationFailed => write!(f, "configuration failed"),
            Self::AdvertisingFailure => write!(f, "ble advertising failed"),
            Self::ConnectionClosed => write!(f, "connection closed"),
            Self::ReadError => write!(f, "read error"),
            Self::WriteError => write!(f, "write error"),
            Self::Other => write!(f, "other error"),
            Self::Unknown => write!(f, "unknown error"),
        }
    }
}

impl From<BleError> for Error {
    fn from(err: BleError) -> Error {
        Error::new(Origin::Transport, Kind::Io, err)
    }
}
