use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Represents the failures that can occur in
/// an Ockam vault sync core
#[derive(Clone, Copy, Debug)]
pub enum VaultSyncCoreError {
    /// Invalid response type.
    InvalidResponseType = 1,
}

impl ockam_core::compat::error::Error for VaultSyncCoreError {}
impl core::fmt::Display for VaultSyncCoreError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidResponseType => write!(f, "invalid response type"),
        }
    }
}

impl From<VaultSyncCoreError> for Error {
    fn from(err: VaultSyncCoreError) -> Self {
        let kind = match err {
            VaultSyncCoreError::InvalidResponseType => Kind::Invalid,
        };

        Error::new(Origin::Vault, kind, err)
    }
}
