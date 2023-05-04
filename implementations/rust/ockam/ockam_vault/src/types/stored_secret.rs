use crate::{Secret, SecretAttributes, VaultError};
use ockam_core::Result;
use serde::{Deserialize, Serialize};

/// Stored secret: binary data + secret metadata
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct StoredSecret {
    secret: Secret,
    attributes: SecretAttributes,
}

impl StoredSecret {
    /// Create a new stored secret
    pub(crate) fn new(secret: Secret, attributes: SecretAttributes) -> Self {
        StoredSecret { secret, attributes }
    }

    /// Create a new stored secret and check the secret length
    pub fn create(secret: Secret, attributes: SecretAttributes) -> Result<Self> {
        Self::check(&secret, &attributes)?;
        Ok(StoredSecret::new(secret, attributes))
    }

    /// Secret's Attributes
    pub fn attributes(&self) -> SecretAttributes {
        self.attributes
    }

    /// Get the secret part of this stored secret
    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    /// Check if the length of the secret is the same as the length prescribed by the attributes
    fn check(secret: &Secret, attributes: &SecretAttributes) -> Result<()> {
        // the secret must be at least the length mentioned in the attributes
        // In the case of a NistP256 secret it might contain additional metadata (algorithm, version, etc...)
        if secret.length() < attributes.length() as usize {
            Err(VaultError::InvalidSecretLength(
                attributes.secret_type(),
                secret.length(),
                attributes.length(),
            )
            .into())
        } else {
            Ok(())
        }
    }
}
