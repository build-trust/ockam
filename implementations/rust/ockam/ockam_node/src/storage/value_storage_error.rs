use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Represents the failures that can occur when storing values
#[derive(Clone, Debug)]
pub enum ValueStorageError {
    /// IO error
    StorageError,
    /// Invalid Storage data
    InvalidStorageData(String),
}

impl ockam_core::compat::error::Error for ValueStorageError {}

impl core::fmt::Display for ValueStorageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::StorageError => write!(f, "invalid storage"),
            Self::InvalidStorageData(e) => write!(f, "invalid storage data {:?}", e),
        }
    }
}

impl From<ValueStorageError> for Error {
    #[track_caller]
    fn from(err: ValueStorageError) -> Self {
        Error::new(Origin::Vault, Kind::Invalid, err)
    }
}
