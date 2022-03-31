use ockam_core::{
    error::{
        code::{ErrorCode, Kind, Origin},
        Error2,
    },
    thiserror,
};

/// Represents the failures that can occur in
/// an Ockam vault sync core
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum VaultSyncCoreError {
    /// Invalid response type.
    #[error("invalid response type")]
    InvalidResponseType = 1,
}

impl From<VaultSyncCoreError> for Error2 {
    fn from(err: VaultSyncCoreError) -> Self {
        let kind = match err {
            VaultSyncCoreError::InvalidResponseType => Kind::Invalid,
        };

        Error2::new(ErrorCode::new(Origin::Vault, kind), err)
    }
}
