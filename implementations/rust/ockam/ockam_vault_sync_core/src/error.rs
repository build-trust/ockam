use ockam_core::Error;

/// Represents the failures that can occur in
/// an Ockam vault sync core
#[derive(Clone, Copy, Debug)]
pub enum VaultSyncCoreError {
    /// Invalid response type.
    InvalidResponseType = 1,
}

impl VaultSyncCoreError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 17_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_VAULT_SYNC_CORE";
}

impl From<VaultSyncCoreError> for Error {
    fn from(err: VaultSyncCoreError) -> Self {
        Self::new(
            VaultSyncCoreError::DOMAIN_CODE + (err as u32),
            ockam_core::compat::format!("{}::{:?}", module_path!(), err),
        )
    }
}
