use ockam_core::{
    errcode::{Kind, Origin},
    thiserror, Error,
};

/// Represents the failures that can occur in
/// an Ockam vault sync core
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum VaultSyncCoreError {
    /// Invalid response type.
    #[error("invalid response type")]
    InvalidResponseType = 1,
}

impl From<VaultSyncCoreError> for Error {
    fn from(err: VaultSyncCoreError) -> Self {
        let kind = match err {
            VaultSyncCoreError::InvalidResponseType => Kind::Invalid,
        };

        Error::new(Origin::Vault, kind, err)
    }
}
