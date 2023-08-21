use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// A Bluetooth Low Energy connection worker specific error type
#[derive(Clone, Debug)]
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
    ReadError,
    WriteError,
    UnexpectedCallback,
    UnexpectedCharacteristic,
    NoSuchCharacteristic,
    RuntimeError(String),
    Other(String),
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
            Self::ReadError => write!(f, "read error"),
            Self::WriteError => write!(f, "write error"),
            Self::UnexpectedCallback => write!(f, "unexpected callback"),
            Self::UnexpectedCharacteristic => write!(f, "unexpected characteristic"),
            Self::NoSuchCharacteristic => write!(f, "no such characteristic"),
            Self::RuntimeError(s) => write!(f, "runtime error {:?}", s),
            Self::Other(s) => write!(f, "other error {:?}", s),
        }
    }
}

impl From<BleError> for Error {
    #[track_caller]
    fn from(err: BleError) -> Error {
        Error::new(Origin::Transport, Kind::Io, err)
    }
}
