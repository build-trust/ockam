use ockam_core::compat::sync::Arc;
use ockam_core::{Error, Result};

use crate::models::{
    Identifier, PurposeKeyAttestation, PurposeKeyAttestationData, PurposePublicKey, VersionedData,
};
use crate::purpose_keys::storage::PurposeKeysRepository;
use crate::{
    CredentialPurposeKey, CredentialPurposeKeyBuilder, IdentitiesKeys, IdentitiesReader, Identity,
    IdentityError, Purpose, PurposeKeyVerification, SecureChannelPurposeKey,
    SecureChannelPurposeKeyBuilder, TimestampInSeconds, Vault,
};

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct PurposeKeyCreation {
    vault: Vault,
    identities_reader: Arc<dyn IdentitiesReader>,
    identity_keys: Arc<IdentitiesKeys>,
    repository: Arc<dyn PurposeKeysRepository>,
}

impl PurposeKeyCreation {
    /// Constructor.
    pub(crate) fn new(
        vault: Vault,
        identities_reader: Arc<dyn IdentitiesReader>,
        identity_keys: Arc<IdentitiesKeys>,
        repository: Arc<dyn PurposeKeysRepository>,
    ) -> Self {
        Self {
            vault,
            identities_reader,
            identity_keys,
            repository,
        }
    }

    /// Return [`PurposeKeysRepository`] instance
    pub fn repository(&self) -> Arc<dyn PurposeKeysRepository> {
        self.repository.clone()
    }

    /// Create [`PurposeKeyVerification`]
    pub fn purpose_keys_verification(&self) -> Arc<PurposeKeyVerification> {
        Arc::new(PurposeKeyVerification::new(
            self.vault.verifying_vault.clone(),
            self.identities_reader.clone(),
        ))
    }

    /// Get an instance of [`PurposeKeyBuilder`]
    pub fn secure_channel_purpose_key_builder(
        &self,
        identifier: &Identifier,
    ) -> SecureChannelPurposeKeyBuilder {
        SecureChannelPurposeKeyBuilder::new(
            Arc::new(Self::new(
                self.vault.clone(),
                self.identities_reader.clone(),
                self.identity_keys.clone(),
                self.repository.clone(),
            )),
            identifier.clone(),
        )
    }

    /// Get an instance of [`PurposeKeyBuilder`]
    pub fn credential_purpose_key_builder(
        &self,
        identifier: &Identifier,
    ) -> CredentialPurposeKeyBuilder {
        CredentialPurposeKeyBuilder::new(
            Arc::new(Self::new(
                self.vault.clone(),
                self.identities_reader.clone(),
                self.identity_keys.clone(),
                self.repository.clone(),
            )),
            identifier.clone(),
        )
    }

    /// Return the [`Vault`]
    pub fn vault(&self) -> &Vault {
        &self.vault
    }
}

impl PurposeKeyCreation {
    /// Create a [`PurposeKey`]
    pub async fn create_secure_channel_purpose_key(
        &self,
        identifier: &Identifier,
    ) -> Result<SecureChannelPurposeKey> {
        let builder = self.secure_channel_purpose_key_builder(identifier);
        builder.build().await
    }

    /// Create a [`PurposeKey`]
    pub async fn create_credential_purpose_key(
        &self,
        identifier: &Identifier,
    ) -> Result<CredentialPurposeKey> {
        let builder = self.credential_purpose_key_builder(identifier);
        builder.build().await
    }

    /// Attest a Purpose Key
    pub async fn attest_purpose_key(
        &self,
        identifier: Identifier,
        public_key: PurposePublicKey,
        created_at: TimestampInSeconds,
        expires_at: TimestampInSeconds,
    ) -> Result<(PurposeKeyAttestation, PurposeKeyAttestationData)> {
        let identity_change_history = self.identities_reader.get_identity(&identifier).await?;
        let identity = Identity::import_from_change_history(
            Some(&identifier),
            identity_change_history,
            self.vault.verifying_vault.clone(),
        )
        .await?;

        let attestation_data = PurposeKeyAttestationData {
            subject: identifier,
            subject_latest_change_hash: identity.latest_change_hash()?.clone(),
            public_key,
            created_at,
            expires_at,
        };

        let attestation_data_binary = minicbor::to_vec(&attestation_data)?;

        let versioned_data = VersionedData {
            version: 1,
            data: attestation_data_binary,
        };
        let versioned_data = minicbor::to_vec(&versioned_data)?;

        let versioned_data_hash = self.vault.verifying_vault.sha256(&versioned_data).await?;

        let signing_key = self.identity_keys.get_secret_key(&identity).await?;
        let signature = self
            .vault
            .identity_vault
            .sign(&signing_key, &versioned_data_hash.0)
            .await?;
        let signature = signature.into();

        let attestation = PurposeKeyAttestation {
            data: versioned_data,
            signature,
        };

        Ok((attestation, attestation_data))
    }

    /// Will try to get own Purpose Key from the repository, if that doesn't succeed - new one
    /// will be generated
    pub async fn get_or_create_secure_channel_purpose_key(
        &self,
        identifier: &Identifier,
    ) -> Result<SecureChannelPurposeKey> {
        let existent_key = async {
            let purpose_key_attestation = self
                .repository
                .get_purpose_key(identifier, Purpose::SecureChannel)
                .await?;

            let purpose_key = self
                .import_secure_channel_purpose_key(&purpose_key_attestation)
                .await?;

            Ok::<SecureChannelPurposeKey, Error>(purpose_key)
        }
        .await;

        match existent_key {
            Ok(purpose_key) => Ok(purpose_key),
            // TODO: Should it be customizable?
            Err(_) => self.create_secure_channel_purpose_key(identifier).await,
        }
    }

    /// Will try to get own Purpose Key from the repository, if that doesn't succeed - new one
    /// will be generated
    pub async fn get_or_create_credential_purpose_key(
        &self,
        identifier: &Identifier,
    ) -> Result<CredentialPurposeKey> {
        let existent_key = async {
            let purpose_key_attestation = self
                .repository
                .get_purpose_key(identifier, Purpose::Credentials)
                .await?;

            let purpose_key = self
                .import_credential_purpose_key(&purpose_key_attestation)
                .await?;

            Ok::<CredentialPurposeKey, Error>(purpose_key)
        }
        .await;

        match existent_key {
            Ok(purpose_key) => Ok(purpose_key),
            // TODO: Should it be customizable?
            Err(_) => self.create_credential_purpose_key(identifier).await,
        }
    }

    /// Get own Purpose Key from the repository
    pub async fn get_secure_channel_purpose_key(
        &self,
        identifier: &Identifier,
    ) -> Result<SecureChannelPurposeKey> {
        let purpose_key_attestation = self
            .repository
            .get_purpose_key(identifier, Purpose::SecureChannel)
            .await?;

        self.import_secure_channel_purpose_key(&purpose_key_attestation)
            .await
    }

    /// Get own Purpose Key from the repository
    pub async fn get_credential_purpose_key(
        &self,
        identifier: &Identifier,
    ) -> Result<CredentialPurposeKey> {
        let purpose_key_attestation = self
            .repository
            .get_purpose_key(identifier, Purpose::Credentials)
            .await?;

        self.import_credential_purpose_key(&purpose_key_attestation)
            .await
    }

    /// Import own [`PurposeKey`] from its [`PurposeKeyAttestation`]
    /// It's assumed that the corresponding secret exists in the Vault
    pub async fn import_secure_channel_purpose_key(
        &self,
        attestation: &PurposeKeyAttestation,
    ) -> Result<SecureChannelPurposeKey> {
        let purpose_key_data = self
            .purpose_keys_verification()
            .verify_purpose_key_attestation(None, attestation)
            .await?;

        let (key_id, public_key) = match purpose_key_data.public_key.clone() {
            PurposePublicKey::SecureChannelStatic(public_key) => {
                let key = self
                    .vault
                    .secure_channel_vault
                    .get_x25519_secret_key_handle(&public_key)
                    .await?;
                (key, public_key)
            }
            PurposePublicKey::CredentialSigning(_public_key) => {
                return Err(IdentityError::InvalidKeyType.into());
            }
        };

        let purpose_key = SecureChannelPurposeKey::new(
            purpose_key_data.subject.clone(),
            key_id,
            public_key,
            purpose_key_data,
            attestation.clone(),
        );

        Ok(purpose_key)
    }

    /// Import own [`PurposeKey`] from its [`PurposeKeyAttestation`]
    /// It's assumed that the corresponding secret exists in the Vault
    pub async fn import_credential_purpose_key(
        &self,
        attestation: &PurposeKeyAttestation,
    ) -> Result<CredentialPurposeKey> {
        let purpose_key_data = self
            .purpose_keys_verification()
            .verify_purpose_key_attestation(None, attestation)
            .await?;

        let (key_id, public_key) = match purpose_key_data.public_key.clone() {
            PurposePublicKey::SecureChannelStatic(_public_key) => {
                return Err(IdentityError::InvalidKeyType.into());
            }
            PurposePublicKey::CredentialSigning(public_key) => {
                let public_key = public_key.into();
                let key = self
                    .vault
                    .credential_vault
                    .get_secret_key_handle(&public_key)
                    .await?;
                (key, public_key)
            }
        };

        let purpose_key = CredentialPurposeKey::new(
            purpose_key_data.subject.clone(),
            key_id,
            public_key,
            purpose_key_data,
            attestation.clone(),
        );

        Ok(purpose_key)
    }
}
