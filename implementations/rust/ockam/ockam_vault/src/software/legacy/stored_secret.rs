use crate::software::legacy::{Secret, SecretAttributes};
use crate::{
    ECDSASHA256CurveP256SecretKey, EdDSACurve25519SecretKey, SigningSecret, VaultError,
    X25519SecretKey,
};
use ockam_core::{Error, Result};
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

    /// Get the secret part of this stored secret
    pub fn take_secret(self) -> Secret {
        self.secret
    }

    /// Check if the length of the secret is the same as the length prescribed by the attributes
    fn check(secret: &Secret, attributes: &SecretAttributes) -> Result<()> {
        // the secret must be equal to length mentioned in the attributes
        if secret.length() != attributes.length() as usize {
            Err(VaultError::InvalidSecretLength.into())
        } else {
            Ok(())
        }
    }
}

impl From<SigningSecret> for StoredSecret {
    fn from(value: SigningSecret) -> Self {
        let (secret, attributes) = match value {
            SigningSecret::EdDSACurve25519(value) => {
                (value.key().to_vec(), SecretAttributes::Ed25519)
            }
            SigningSecret::ECDSASHA256CurveP256(value) => {
                (value.key().to_vec(), SecretAttributes::NistP256)
            }
        };

        let secret = Secret::new(secret);

        Self::new(secret, attributes)
    }
}

impl TryFrom<StoredSecret> for SigningSecret {
    type Error = Error;

    fn try_from(value: StoredSecret) -> Result<Self, Self::Error> {
        match &value.attributes {
            SecretAttributes::Ed25519 => {
                let secret = value.secret;

                let secret = secret
                    .as_ref()
                    .try_into()
                    .map_err(|_| VaultError::InvalidSecretLength)?;
                let secret = EdDSACurve25519SecretKey::new(secret);

                Ok(Self::EdDSACurve25519(secret))
            }
            SecretAttributes::NistP256 => {
                let secret = value.secret;

                let secret = secret
                    .as_ref()
                    .try_into()
                    .map_err(|_| VaultError::InvalidSecretLength)?;
                let secret = ECDSASHA256CurveP256SecretKey::new(secret);

                Ok(Self::ECDSASHA256CurveP256(secret))
            }

            SecretAttributes::X25519
            | SecretAttributes::Buffer(_)
            | SecretAttributes::Aes128
            | SecretAttributes::Aes256 => Err(VaultError::InvalidKeyType.into()),
        }
    }
}

impl From<X25519SecretKey> for StoredSecret {
    fn from(value: X25519SecretKey) -> Self {
        let secret = Secret::new(value.key().to_vec());

        Self::new(secret, SecretAttributes::X25519)
    }
}

impl TryFrom<StoredSecret> for X25519SecretKey {
    type Error = Error;

    fn try_from(value: StoredSecret) -> Result<Self, Self::Error> {
        match &value.attributes {
            SecretAttributes::X25519 => {
                let secret = value.secret;

                let secret = secret
                    .as_ref()
                    .try_into()
                    .map_err(|_| VaultError::InvalidSecretLength)?;

                Ok(Self::new(secret))
            }

            SecretAttributes::Ed25519
            | SecretAttributes::NistP256
            | SecretAttributes::Buffer(_)
            | SecretAttributes::Aes128
            | SecretAttributes::Aes256 => Err(VaultError::InvalidKeyType.into()),
        }
    }
}
