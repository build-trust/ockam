use crate::{SoftwareVault, VaultError};
use ockam_vault_core::ErrorVault;

impl ErrorVault for SoftwareVault {
    fn error_domain() -> &'static str {
        VaultError::DOMAIN_NAME
    }
}
