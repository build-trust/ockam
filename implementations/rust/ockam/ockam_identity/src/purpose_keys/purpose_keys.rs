use super::super::models::{
    Ed25519Signature, Identifier, PurposeKeyAttestation, PurposeKeyAttestationData,
    PurposeKeyAttestationSignature, PurposePublicKey, VersionedData,
};
use super::super::utils::{add_seconds, now};
use super::super::{
    IdentitiesKeys, IdentitiesReader, Identity, IdentityError, Purpose, PurposeKey,
};
use super::storage::PurposeKeysRepository;

use ockam_core::compat::sync::Arc;
use ockam_core::{Error, Result};
use ockam_vault::{SecretAttributes, SecretType, Signature, Vault, PublicKey};

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct PurposeKeys {
    vault: Vault,
    identities_reader: Arc<dyn IdentitiesReader>,
    identity_keys: Arc<IdentitiesKeys>,
    repository: Arc<dyn PurposeKeysRepository>,
}

impl PurposeKeys {
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
}

impl PurposeKeys {
    /// Create a [`PurposeKey`]
    pub async fn create_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<PurposeKey> {
        // TODO: Check if such key already exists and rewrite it correctly (also delete from the Vault)

        let identity_change_history = self.identities_reader.get_identity(identifier).await?;
        let identity = Identity::import_from_change_history(
            Some(identifier),
            identity_change_history,
            self.vault.verifying_vault.clone(),
        )
        .await?;

        // FIXME
        let (secret_key, public_key) = match &purpose {
            Purpose::SecureChannel => {
                let secret_key = self
                    .vault
                    .secure_channel_vault
                    .generate_static_secret(SecretAttributes::X25519)
                    .await?;
                let public_key = self
                    .vault
                    .secure_channel_vault
                    .get_public_key(&secret_key)
                    .await?;
                let public_key =
                    PurposePublicKey::SecureChannelStaticKey(public_key.try_into().unwrap()); // FIXME
                (secret_key, public_key)
            }
            Purpose::Credentials => {
                let secret_key = self
                    .vault
                    .signing_vault
                    .generate_key(SecretAttributes::Ed25519)
                    .await?;
                let public_key = self.vault.signing_vault.get_public_key(&secret_key).await?;
                let public_key =
                    PurposePublicKey::CredentialSigningKey(public_key.try_into().unwrap()); // FIXME
                (secret_key, public_key)
            }
        };

        let created_at = now()?;
        // TODO: allow customizing ttl
        // TODO: check if expiration is before the purpose key expiration
        let five_years = 5 * 365 * 24 * 60 * 60;
        let expires_at = add_seconds(&created_at, five_years);

        let purpose_key_attestation_data = PurposeKeyAttestationData {
            subject: identity.identifier().clone(),
            subject_latest_change_hash: identity.latest_change_hash()?.clone(),
            public_key,
            created_at,
            expires_at,
        };

        let purpose_key_attestation_data_binary = minicbor::to_vec(&purpose_key_attestation_data)?;

        let versioned_data = VersionedData {
            version: 1,
            data: purpose_key_attestation_data_binary,
        };
        let versioned_data = minicbor::to_vec(&versioned_data)?;

        let versioned_data_hash = self.vault.verifying_vault.sha256(&versioned_data).await?;

        let signing_key = self.identity_keys.get_secret_key(&identity).await?;
        let signature = self
            .vault
            .signing_vault
            .sign(&signing_key, &versioned_data_hash)
            .await?;
        let signature = Ed25519Signature(signature.as_ref().try_into().unwrap()); // FIXME
        let signature = PurposeKeyAttestationSignature::Ed25519Signature(signature);

        let attestation = PurposeKeyAttestation {
            data: versioned_data,
            signature,
        };

        self.repository
            .set_purpose_key(identifier, purpose, &attestation)
            .await?;

        let purpose_key = PurposeKey::new(
            identifier.clone(),
            secret_key,
            SecretType::Ed25519,
            purpose,
            purpose_key_attestation_data,
            attestation,
        );

        Ok(purpose_key)
    }

    /// Attest a given PublicKey as a [`PurposeKey`]
    pub async fn attest_purpose_key(
        &self,
        identifier: &Identifier,
        _purpose: Purpose,
        to_attest: PublicKey,
    ) -> Result<PurposeKeyAttestation> {
        // TODO: Check if such key already exists and rewrite it correctly (also delete from the Vault)

        let identity_change_history = self.identities_reader.get_identity(identifier).await?;
        let identity = Identity::import_from_change_history(
            Some(identifier),
            identity_change_history,
            self.vault.verifying_vault.clone(),
        )
        .await?;

        let public_key = PurposePublicKey::SecureChannelStaticKey(to_attest.try_into().unwrap()); // FIXME
        let created_at = now()?;
        // TODO: allow customizing ttl
        // TODO: check if expiration is before the purpose key expiration
        let five_years = 5 * 365 * 24 * 60 * 60;
        let expires_at = add_seconds(&created_at, five_years);

        let purpose_key_attestation_data = PurposeKeyAttestationData {
            subject: identity.identifier().clone(),
            subject_latest_change_hash: identity.latest_change_hash()?.clone(),
            public_key,
            created_at,
            expires_at,
        };

        let purpose_key_attestation_data_binary = minicbor::to_vec(&purpose_key_attestation_data)?;

        let versioned_data = VersionedData {
            version: 1,
            data: purpose_key_attestation_data_binary,
        };
        let versioned_data = minicbor::to_vec(&versioned_data)?;

        let versioned_data_hash = self.vault.verifying_vault.sha256(&versioned_data).await?;

        let signing_key = self.identity_keys.get_secret_key(&identity).await?;
        let signature = self
            .vault
            .signing_vault
            .sign(&signing_key, &versioned_data_hash)
            .await?;
        let signature = Ed25519Signature(signature.as_ref().try_into().unwrap()); // FIXME
        let signature = PurposeKeyAttestationSignature::Ed25519Signature(signature);

        let attestation = PurposeKeyAttestation {
            data: versioned_data,
            signature,
        };
        Ok(attestation)
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

    /// Verify a [`PurposeKeyAttestation`]
    pub async fn verify_purpose_key_attestation(
        &self,
        expected_subject: Option<&Identifier>,
        attestation: &PurposeKeyAttestation,
    ) -> Result<PurposeKeyAttestationData> {
        let versioned_data_hash = self.vault.verifying_vault.sha256(&attestation.data).await?;

        let versioned_data = attestation.get_versioned_data()?;

        if versioned_data.version != 1 {
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        let purpose_key_data = PurposeKeyAttestationData::get_data(&versioned_data)?;

        if let Some(expected_subject) = expected_subject {
            if expected_subject != &purpose_key_data.subject {
                // We expected purpose key that belongs to someone else
                return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
            }
        }

        
        let change_history = self
            .identities_reader
            .get_identity(&purpose_key_data.subject)
            .await?;
        let identity = Identity::import_from_change_history(
            Some(&purpose_key_data.subject),
            change_history,
            self.vault.verifying_vault.clone(),
        )
        .await?;

        let latest_change = identity.get_latest_change()?;

        // TODO: We should inspect purpose_key_data.subject_latest_change_hash, the possibilities are:
        //     1) It's equal to the latest Change we know about, this is the default case and
        //        this is the only case that the code below handles currently
        //     2) We haven't yet discovered that new Change, therefore we can't verify such PurposeKey
        //     3) It references previous Change from the known to us history, we might accept such
        //        PurposeKey, but not if the next Change has revoke_all_purpose_keys == true
        //     4) It references Change even older. IMO we shouldn't accept such PurposeKeys

        if &purpose_key_data.subject_latest_change_hash != latest_change.change_hash() {
            // Only verifying with the latest key is currently implemented, see the `TODO` above
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        if purpose_key_data.expires_at > latest_change.data().expires_at {
            // PurposeKey validity time range should be inside the identity key validity time range
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        if purpose_key_data.created_at < latest_change.data().created_at {
            // PurposeKey validity time range should be inside the identity key validity time range
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        let now = now()?;

        if purpose_key_data.created_at > now {
            // PurposeKey can't be created in the future
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        if purpose_key_data.expires_at < now {
            // PurposeKey expired
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        let identity_public_key = latest_change.primary_public_key();

        let signature = if let PurposeKeyAttestationSignature::Ed25519Signature(signature) =
            &attestation.signature
        {
            Signature::new(signature.0.to_vec())
        } else {
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        };

        if !self
            .vault
            .verifying_vault
            .verify(identity_public_key, &versioned_data_hash, &signature)
            .await?
        {
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        Ok(purpose_key_data)
    }

    /// Import own [`PurposeKey`] from its [`PurposeKeyAttestation`]
    /// It's assumed that the corresponding secret exists in the Vault
    pub async fn import_purpose_key(
        &self,
        attestation: &PurposeKeyAttestation,
    ) -> Result<PurposeKey> {
        let purpose_key_data = self
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
                    .signing_vault
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

#[cfg(test)]
mod tests {
    use super::super::super::{identities, Purpose};
    use super::*;

    #[tokio::test]
    async fn create_purpose_keys() -> Result<()> {
        let identities = identities();
        let identities_creation = identities.identities_creation();
        let purpose_keys = identities.purpose_keys();

        let identity = identities_creation.create_identity().await?;
        let credentials_key = purpose_keys
            .create_purpose_key(identity.identifier(), Purpose::Credentials)
            .await?;
        let secure_channel_key = purpose_keys
            .create_purpose_key(identity.identifier(), Purpose::SecureChannel)
            .await?;

        let credentials_key = purpose_keys
            .verify_purpose_key_attestation(
                Some(identity.identifier()),
                credentials_key.attestation(),
            )
            .await?;
        let secure_channel_key = purpose_keys
            .verify_purpose_key_attestation(
                Some(identity.identifier()),
                secure_channel_key.attestation(),
            )
            .await?;

        assert_eq!(identity.identifier(), &credentials_key.subject);
        assert_eq!(identity.identifier(), &secure_channel_key.subject);

        Ok(())
    }

    #[tokio::test]
    async fn test_purpose_keys_are_persisted() -> Result<()> {
        let identities = identities();
        let identities_creation = identities.identities_creation();
        let purpose_keys = identities.purpose_keys();

        let identity = identities_creation.create_identity().await?;

        let credentials_key = purpose_keys
            .create_purpose_key(identity.identifier(), Purpose::Credentials)
            .await?;

        assert!(purpose_keys
            .repository()
            .retrieve_purpose_key(identity.identifier(), Purpose::Credentials)
            .await?
            .is_some());
        assert!(purpose_keys
            .repository()
            .retrieve_purpose_key(identity.identifier(), Purpose::SecureChannel)
            .await?
            .is_none());

        let secure_channel_key = purpose_keys
            .create_purpose_key(identity.identifier(), Purpose::SecureChannel)
            .await?;

        let key = purpose_keys
            .repository()
            .retrieve_purpose_key(identity.identifier(), Purpose::Credentials)
            .await?
            .unwrap();
        purpose_keys
            .verify_purpose_key_attestation(Some(identity.identifier()), &key)
            .await
            .unwrap();
        assert_eq!(&key, credentials_key.attestation());

        let key = purpose_keys
            .repository()
            .retrieve_purpose_key(identity.identifier(), Purpose::SecureChannel)
            .await?
            .unwrap();
        purpose_keys
            .verify_purpose_key_attestation(Some(identity.identifier()), &key)
            .await
            .unwrap();
        assert_eq!(&key, secure_channel_key.attestation());

        let credentials_key2 = purpose_keys
            .create_purpose_key(identity.identifier(), Purpose::Credentials)
            .await?;

        let key = purpose_keys
            .repository()
            .retrieve_purpose_key(identity.identifier(), Purpose::Credentials)
            .await?
            .unwrap();
        purpose_keys
            .verify_purpose_key_attestation(Some(identity.identifier()), &key)
            .await
            .unwrap();
        assert_eq!(&key, credentials_key2.attestation());

        Ok(())
    }
}
