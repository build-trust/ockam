use ockam_core::compat::sync::Arc;
use ockam_core::{Error, Result};
use ockam_vault::SecretType;

use crate::models::{
    Identifier, PurposeKeyAttestation, PurposeKeyAttestationData, PurposeKeyAttestationSignature,
    PurposePublicKey, VersionedData,
};
use crate::purpose_keys::storage::PurposeKeysRepository;
use crate::{
    IdentitiesKeys, IdentitiesReader, Identity, IdentityError, Purpose, PurposeKey,
    PurposeKeyBuilder, PurposeKeyOptions, PurposeKeysVerification, Vault,
};

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct PurposeKeysCreation {
    vault: Vault,
    identities_reader: Arc<dyn IdentitiesReader>,
    identity_keys: Arc<IdentitiesKeys>,
    repository: Arc<dyn PurposeKeysRepository>,
}

impl PurposeKeysCreation {
    /// Create a new identities module
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

    /// Create [`PurposeKeysVerification`]
    pub fn purpose_keys_verification(&self) -> Arc<PurposeKeysVerification> {
        Arc::new(PurposeKeysVerification::new(
            self.vault.verifying_vault.clone(),
            self.identities_reader.clone(),
        ))
    }

    /// Get an instance of [`PurposeKeyBuilder`]
    pub fn purpose_key_builder(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> PurposeKeyBuilder {
        PurposeKeyBuilder::new(
            Arc::new(Self::new(
                self.vault.clone(),
                self.identities_reader.clone(),
                self.identity_keys.clone(),
                self.repository.clone(),
            )),
            identifier.clone(),
            purpose,
        )
    }

    /// Return the [`Vault`]
    pub fn vault(&self) -> &Vault {
        &self.vault
    }
}

impl PurposeKeysCreation {
    /// Create a [`PurposeKey`]
    pub async fn create_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<PurposeKey> {
        let builder = self.purpose_key_builder(identifier, purpose);
        builder.build().await
    }

    /// Create a [`PurposeKey`]
    pub async fn create_purpose_key_with_options(
        &self,
        options: PurposeKeyOptions,
    ) -> Result<PurposeKey> {
        // TODO: Check if such key already exists and rewrite it correctly (also delete from the Vault)

        match options.purpose {
            Purpose::SecureChannel => match options.stype {
                SecretType::X25519 => {}

                SecretType::Buffer
                | SecretType::Aes
                | SecretType::Ed25519
                | SecretType::NistP256 => {
                    return Err(IdentityError::InvalidKeyType.into());
                }
            },
            Purpose::Credentials => match options.stype {
                SecretType::Ed25519 | SecretType::NistP256 => {}

                SecretType::Buffer | SecretType::Aes | SecretType::X25519 => {
                    return Err(IdentityError::InvalidKeyType.into());
                }
            },
        }

        let identifier = options.identifier;
        let identity_change_history = self.identities_reader.get_identity(&identifier).await?;
        let identity = Identity::import_from_change_history(
            Some(&identifier),
            identity_change_history,
            self.vault.verifying_vault.clone(),
        )
        .await?;

        let secret_key = options.key;
        let public_key = match &options.purpose {
            Purpose::SecureChannel => {
                let public_key = self
                    .vault
                    .secure_channel_vault
                    .get_public_key(&secret_key)
                    .await?;
                PurposePublicKey::SecureChannelStaticKey(
                    public_key
                        .try_into()
                        .map_err(|_| IdentityError::InvalidKeyType)?,
                )
            }
            Purpose::Credentials => {
                let public_key = self
                    .vault
                    .credential_vault
                    .get_public_key(&secret_key)
                    .await?;
                PurposePublicKey::CredentialSigningKey(
                    public_key
                        .try_into()
                        .map_err(|_| IdentityError::InvalidKeyType)?,
                )
            }
        };

        let purpose_key_attestation_data = PurposeKeyAttestationData {
            subject: identity.identifier().clone(),
            subject_latest_change_hash: identity.latest_change_hash()?.clone(),
            public_key,
            created_at: options.created_at,
            expires_at: options.expires_at,
        };

        let purpose_key_attestation_data_binary = minicbor::to_vec(&purpose_key_attestation_data)?;

        let versioned_data = VersionedData {
            version: 1,
            data: purpose_key_attestation_data_binary,
        };
        let versioned_data = minicbor::to_vec(&versioned_data)?;

        let versioned_data_hash = self.vault.verifying_vault.sha256(&versioned_data).await?;

        let signing_key = self.identity_keys.get_secret_key(&identity).await?;
        // TODO: Optimize
        let public_key = self
            .vault
            .identity_vault
            .get_public_key(&signing_key)
            .await?;
        let signature = self
            .vault
            .identity_vault
            .sign(&signing_key, &versioned_data_hash)
            .await?;
        let signature =
            PurposeKeyAttestationSignature::try_from_signature(signature, public_key.stype())?;

        let attestation = PurposeKeyAttestation {
            data: versioned_data,
            signature,
        };

        self.repository
            .set_purpose_key(&identifier, options.purpose, &attestation)
            .await?;

        let purpose_key = PurposeKey::new(
            identifier,
            secret_key,
            SecretType::Ed25519,
            options.purpose,
            purpose_key_attestation_data,
            attestation,
        );

        Ok(purpose_key)
    }

    /// Will try to get own Purpose Key from the repository, if that doesn't succeed - new one
    /// will be generated
    pub async fn get_or_create_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<PurposeKey> {
        let existent_key = async {
            let purpose_key_attestation =
                self.repository.get_purpose_key(identifier, purpose).await?;

            let purpose_key = self.import_purpose_key(&purpose_key_attestation).await?;

            Ok::<PurposeKey, Error>(purpose_key)
        }
        .await;

        match existent_key {
            Ok(purpose_key) => Ok(purpose_key),
            // TODO: Should it be customizable?
            Err(_) => self.create_purpose_key(identifier, purpose).await,
        }
    }

    /// Get own Purpose Key from the repository
    pub async fn get_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<PurposeKey> {
        let purpose_key_attestation = self.repository.get_purpose_key(identifier, purpose).await?;

        self.import_purpose_key(&purpose_key_attestation).await
    }

    /// Import own [`PurposeKey`] from its [`PurposeKeyAttestation`]
    /// It's assumed that the corresponding secret exists in the Vault
    pub async fn import_purpose_key(
        &self,
        attestation: &PurposeKeyAttestation,
    ) -> Result<PurposeKey> {
        let purpose_key_data = self
            .purpose_keys_verification()
            .verify_purpose_key_attestation(None, attestation)
            .await?;

        let (purpose, key_id) = match purpose_key_data.public_key.clone() {
            PurposePublicKey::SecureChannelStaticKey(public_key) => {
                let key_id = self
                    .vault
                    .secure_channel_vault
                    .get_key_id(&public_key.into())
                    .await?;
                (Purpose::SecureChannel, key_id)
            }
            PurposePublicKey::CredentialSigningKey(public_key) => {
                let key_id = self
                    .vault
                    .credential_vault
                    .get_key_id(&public_key.into())
                    .await?;
                (Purpose::Credentials, key_id)
            }
        };

        let purpose_key = PurposeKey::new(
            purpose_key_data.subject.clone(),
            key_id,
            SecretType::Ed25519,
            purpose,
            purpose_key_data,
            attestation.clone(),
        );

        Ok(purpose_key)
    }
}
