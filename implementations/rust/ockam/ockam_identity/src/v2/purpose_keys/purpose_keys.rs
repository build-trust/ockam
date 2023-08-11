use super::super::models::{
    Ed25519Signature, Identifier, PurposeKeyAttestation, PurposeKeyAttestationData,
    PurposeKeyAttestationSignature, PurposePublicKey, VersionedData,
};
use super::super::utils::{add_seconds, now};
use super::super::{
    IdentitiesKeys, IdentitiesReader, IdentitiesVault, Identity, IdentityError, Purpose, PurposeKey,
};
use super::storage::PurposeKeysRepository;

use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{SecretAttributes, SecretType, Signature, Vault};

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct PurposeKeys {
    vault: Arc<dyn IdentitiesVault>,
    identities_reader: Arc<dyn IdentitiesReader>,
    identity_keys: Arc<IdentitiesKeys>,
    repository: Arc<dyn PurposeKeysRepository>,
}

impl PurposeKeys {
    /// Return the identities vault
    pub fn vault(&self) -> Arc<dyn IdentitiesVault> {
        self.vault.clone()
    }
}

impl PurposeKeys {
    /// Create a new identities module
    pub(crate) fn new(
        vault: Arc<dyn IdentitiesVault>,
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
            self.vault(),
        )
        .await?;

        // FIXME
        let secret_attributes = match &purpose {
            Purpose::SecureChannel => SecretAttributes::X25519,
            Purpose::Credentials => SecretAttributes::Ed25519,
        };
        let secret_key = self
            .vault
            .create_ephemeral_secret(secret_attributes)
            .await?;

        let public_key = self.vault.get_public_key(&secret_key).await?;

        let public_key = match &purpose {
            Purpose::SecureChannel => {
                PurposePublicKey::SecureChannelStaticKey(public_key.try_into().unwrap())
            }
            Purpose::Credentials => {
                PurposePublicKey::CredentialSigningKey(public_key.try_into().unwrap())
            }
        };

        let created_at = now()?;
        let expires_at = add_seconds(&created_at, 60); // FIXME

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

        let versioned_data_hash = Vault::sha256(&versioned_data);

        let signing_key = self.identity_keys.get_secret_key(&identity).await?;
        let signature = self.vault.sign(&signing_key, &versioned_data_hash).await?;
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

    /// Verify a [`PurposeKeyAttestation`]
    pub async fn verify_purpose_key_attestation(
        &self,
        attestation: &PurposeKeyAttestation,
    ) -> Result<PurposeKeyAttestationData> {
        let versioned_data_hash = Vault::sha256(&attestation.data);

        let versioned_data: VersionedData = minicbor::decode(&attestation.data)?;

        if versioned_data.version != 1 {
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        let purpose_key_data: PurposeKeyAttestationData = minicbor::decode(&versioned_data.data)?;

        let change_history = self
            .identities_reader
            .get_identity(&purpose_key_data.subject)
            .await?;
        let identity = Identity::import_from_change_history(
            Some(&purpose_key_data.subject),
            change_history,
            self.vault.clone(),
        )
        .await?;

        let public_key = identity.get_public_key()?;

        let signature = if let PurposeKeyAttestationSignature::Ed25519Signature(signature) =
            &attestation.signature
        {
            Signature::new(signature.0.to_vec())
        } else {
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        };

        if !self
            .vault
            .verify(&public_key, &versioned_data_hash, &signature)
            .await?
        {
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        let now = now()?;

        if purpose_key_data.created_at > now {
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        if purpose_key_data.expires_at < now {
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        // FIXME: purpose_key_data.subject_latest_change_hash;

        Ok(purpose_key_data)
    }

    /// Import own [`PurposeKey`] from its [`PurposeKeyAttestation`]
    /// It's assumed that the corresponding secret exists in the Vault
    pub async fn import_purpose_key(
        &self,
        attestation: &PurposeKeyAttestation,
    ) -> Result<PurposeKey> {
        let purpose_key_data = self.verify_purpose_key_attestation(attestation).await?;

        let (purpose, public_key) = match purpose_key_data.public_key.clone() {
            PurposePublicKey::SecureChannelStaticKey(public_key) => {
                (Purpose::SecureChannel, public_key.into())
            }
            PurposePublicKey::CredentialSigningKey(public_key) => {
                (Purpose::Credentials, public_key.into())
            }
        };

        let key_id = self.vault.get_key_id(&public_key).await?;

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
            .verify_purpose_key_attestation(credentials_key.attestation())
            .await?;
        let secure_channel_key = purpose_keys
            .verify_purpose_key_attestation(secure_channel_key.attestation())
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
            .verify_purpose_key_attestation(&key)
            .await
            .unwrap();
        assert_eq!(&key, credentials_key.attestation());

        let key = purpose_keys
            .repository()
            .retrieve_purpose_key(identity.identifier(), Purpose::SecureChannel)
            .await?
            .unwrap();
        purpose_keys
            .verify_purpose_key_attestation(&key)
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
            .verify_purpose_key_attestation(&key)
            .await
            .unwrap();
        assert_eq!(&key, credentials_key2.attestation());

        Ok(())
    }
}
