use ockam_core::compat::sync::Arc;

use crate::purpose_keys::storage::PurposeKeysRepository;
use crate::{
    IdentitiesCreation, IdentitiesKeys, PurposeKeyCreation, PurposeKeyVerification, Vault,
};

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct PurposeKeys {
    vault: Vault,
    identities_creation: Arc<IdentitiesCreation>,
    identity_keys: Arc<IdentitiesKeys>,
    repository: Arc<dyn PurposeKeysRepository>,
}

impl PurposeKeys {
    /// Create a new identities module
    pub fn new(
        vault: Vault,
        identities_creation: Arc<IdentitiesCreation>,
        identity_keys: Arc<IdentitiesKeys>,
        repository: Arc<dyn PurposeKeysRepository>,
    ) -> Self {
        Self {
            vault,
            identities_creation,
            identity_keys,
            repository,
        }
    }

    /// Return [`PurposeKeysRepository`] instance
    pub fn repository(&self) -> Arc<dyn PurposeKeysRepository> {
        self.repository.clone()
    }

    /// Create [`PurposeKeyCreation`]
    pub fn purpose_keys_creation(&self) -> Arc<PurposeKeyCreation> {
        Arc::new(PurposeKeyCreation::new(
            self.vault.clone(),
            self.identities_creation.clone(),
            self.identity_keys.clone(),
            self.repository.clone(),
        ))
    }

    /// Create [`PurposeKeyVerification`]
    pub fn purpose_keys_verification(&self) -> Arc<PurposeKeyVerification> {
        Arc::new(PurposeKeyVerification::new(
            self.vault.verifying_vault.clone(),
            self.identities_creation.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use ockam_core::Result;

    use crate::{identities, Purpose};

    #[tokio::test]
    async fn create_purpose_keys() -> Result<()> {
        let identities = identities().await?;
        let identities_creation = identities.identities_creation();
        let purpose_keys = identities.purpose_keys();

        let identifier = identities_creation.create_identity().await?;
        let credentials_key = purpose_keys
            .purpose_keys_creation()
            .create_credential_purpose_key(&identifier)
            .await?;
        let secure_channel_key = purpose_keys
            .purpose_keys_creation()
            .create_secure_channel_purpose_key(&identifier)
            .await?;

        let credentials_key = purpose_keys
            .purpose_keys_verification()
            .verify_purpose_key_attestation(Some(&identifier), credentials_key.attestation())
            .await?;
        let secure_channel_key = purpose_keys
            .purpose_keys_verification()
            .verify_purpose_key_attestation(Some(&identifier), secure_channel_key.attestation())
            .await?;

        assert_eq!(identifier, credentials_key.subject);
        assert_eq!(identifier, secure_channel_key.subject);

        Ok(())
    }

    #[tokio::test]
    async fn test_purpose_keys_are_persisted() -> Result<()> {
        let identities = identities().await?;
        let identities_creation = identities.identities_creation();
        let purpose_keys = identities.purpose_keys();

        let identifier = identities_creation.create_identity().await?;

        let credentials_key = purpose_keys
            .purpose_keys_creation()
            .create_credential_purpose_key(&identifier)
            .await?;

        assert!(purpose_keys
            .repository()
            .get_purpose_key(&identifier, Purpose::Credentials)
            .await?
            .is_some());
        assert!(purpose_keys
            .repository()
            .get_purpose_key(&identifier, Purpose::SecureChannel)
            .await?
            .is_none());

        let secure_channel_key = purpose_keys
            .purpose_keys_creation()
            .create_secure_channel_purpose_key(&identifier)
            .await?;

        let key = purpose_keys
            .repository()
            .get_purpose_key(&identifier, Purpose::Credentials)
            .await?
            .unwrap();
        purpose_keys
            .purpose_keys_verification()
            .verify_purpose_key_attestation(Some(&identifier), &key)
            .await
            .unwrap();
        assert_eq!(&key, credentials_key.attestation());

        let key = purpose_keys
            .repository()
            .get_purpose_key(&identifier, Purpose::SecureChannel)
            .await?
            .unwrap();
        purpose_keys
            .purpose_keys_verification()
            .verify_purpose_key_attestation(Some(&identifier), &key)
            .await
            .unwrap();
        assert_eq!(&key, secure_channel_key.attestation());

        let credentials_key2 = purpose_keys
            .purpose_keys_creation()
            .create_credential_purpose_key(&identifier)
            .await?;

        let key = purpose_keys
            .repository()
            .get_purpose_key(&identifier, Purpose::Credentials)
            .await?
            .unwrap();
        purpose_keys
            .purpose_keys_verification()
            .verify_purpose_key_attestation(Some(&identifier), &key)
            .await
            .unwrap();
        assert_eq!(&key, credentials_key2.attestation());

        Ok(())
    }
}
