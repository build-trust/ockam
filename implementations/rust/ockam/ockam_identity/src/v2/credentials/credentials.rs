use super::super::identities::AttributesEntry;
use super::super::models::{
    Attributes, Credential, CredentialData, CredentialSignature, CredentialSigningKey,
    Ed25519Signature, Identifier, PurposeKeyAttestation, PurposeKeyAttestationData,
    PurposePublicKey, VersionedData,
};
use super::super::utils::{add_seconds, now};
use super::super::{
    IdentitiesRepository, IdentitiesVault, Identity, IdentityError, Purpose, PurposeKey,
    PurposeKeys,
};

use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{PublicKey, SecretType, Signature, Vault};

pub struct Credentials {
    vault: Arc<dyn IdentitiesVault>,
    purpose_keys: Arc<PurposeKeys>,
    identities_repository: Arc<dyn IdentitiesRepository>,
}

impl Credentials {
    pub async fn verify_credential(
        &self,
        subject: &Identifier,
        authorities: &[Identifier],
        purpose_key_attestation: &PurposeKeyAttestation,
        credential: &Credential,
    ) -> Result<(CredentialData, PurposeKeyAttestationData)> {
        let purpose_key_data = self
            .purpose_keys
            .verify_purpose_key_attestation(purpose_key_attestation)
            .await?;

        if !authorities.contains(&purpose_key_data.subject) {
            return Err(IdentityError::UnknownAuthority.into());
        }

        let purpose_key = match &purpose_key_data.public_key {
            PurposePublicKey::SecureChannelAuthenticationKey(_) => {
                return Err(IdentityError::InvalidKeyType.into())
            }

            PurposePublicKey::CredentialSigningKey(key) => key,
        };

        let purpose_key = match purpose_key {
            CredentialSigningKey::Ed25519PublicKey(key) => key,
            CredentialSigningKey::P256ECDSAPublicKey(_) => {
                return Err(IdentityError::InvalidKeyType.into())
            }
        };

        let purpose_key = PublicKey::new(purpose_key.0.to_vec(), SecretType::Ed25519);

        let versioned_data_hash = Vault::sha256(&credential.data);

        let signature = match &credential.signature {
            CredentialSignature::Ed25519Signature(signature) => {
                Signature::new(signature.0.to_vec())
            }
            CredentialSignature::P256ECDSASignature(_) => {
                return Err(IdentityError::InvalidKeyType.into())
            }
        };

        if !self
            .vault
            .verify(&purpose_key, &versioned_data_hash, &signature)
            .await?
        {
            return Err(IdentityError::CredentialVerificationFailed.into());
        }

        let versioned_data: VersionedData = minicbor::decode(&credential.data)?;
        if versioned_data.version != 1 {
            return Err(IdentityError::UnknownCredentialVersion.into());
        }

        let credential_data: CredentialData = minicbor::decode(&versioned_data.data)?;

        if credential_data.subject.as_ref() != Some(subject) {
            return Err(IdentityError::CredentialVerificationFailed.into());
        }

        if credential_data.created_at < purpose_key_data.created_at {
            return Err(IdentityError::CredentialVerificationFailed.into());
        }

        if credential_data.expires_at > purpose_key_data.expires_at {
            return Err(IdentityError::CredentialVerificationFailed.into());
        }

        let now = now()?;

        if credential_data.created_at > now {
            return Err(IdentityError::CredentialVerificationFailed.into());
        }

        if credential_data.expires_at < now {
            return Err(IdentityError::CredentialVerificationFailed.into());
        }

        // FIXME: credential_data.subject_latest_change_hash
        // FIXME: Verify if given authority is allowed to issue credentials with given Schema
        // FIXME: Verify if Schema aligns with Attributes

        Ok((credential_data, purpose_key_data))
    }

    /// Create a signed credential based on the given values.
    pub async fn issue_credential(
        &self,
        subject: &Identifier,
        issuer_purpose_key: &PurposeKey,
        subject_attributes: Attributes,
        ttl: Duration,
    ) -> Result<Credential> {
        let subject_change_history = self.identities_repository.get_identity(subject).await?;
        let subject_identity =
            Identity::import_from_change_history(subject_change_history, self.vault.clone())
                .await?;

        let created_at = now()?;
        let expires_at = add_seconds(&created_at, ttl.as_secs());

        let credential_data = CredentialData {
            subject: Some(subject.clone()),
            subject_latest_change_hash: Some(subject_identity.latest_change_hash()?.clone()),
            subject_attributes,
            created_at,
            expires_at,
        };
        let credential_data = minicbor::to_vec(credential_data)?;

        let versioned_data = VersionedData {
            version: 1,
            data: credential_data,
        };
        let versioned_data = minicbor::to_vec(&versioned_data)?;

        let versioned_data_hash = Vault::sha256(&versioned_data);

        if issuer_purpose_key.purpose() != Purpose::Credentials {
            return Err(IdentityError::InvalidKeyType.into());
        }

        if issuer_purpose_key.stype() != SecretType::Ed25519 {
            return Err(IdentityError::InvalidKeyType.into());
        }

        let signature = self
            .vault
            .sign(issuer_purpose_key.key_id(), &versioned_data_hash)
            .await?;
        let signature: Vec<u8> = signature.into();
        let signature = Ed25519Signature(signature.try_into().unwrap());
        let signature = CredentialSignature::Ed25519Signature(signature);

        Ok(Credential {
            data: vec![],
            signature,
        })
    }

    pub async fn receive_presented_credential(
        &self,
        subject: &Identifier,
        authorities: &[Identifier],
        purpose_key_attestation: &PurposeKeyAttestation,
        credential: &Credential,
    ) -> Result<()> {
        let (credential_data, purpose_key_data) = self
            .verify_credential(subject, authorities, purpose_key_attestation, credential)
            .await?;

        self.identities_repository
            .put_attributes(
                subject,
                AttributesEntry::new(
                    credential_data.subject_attributes.map,
                    now()?,
                    Some(credential_data.expires_at),
                    Some(purpose_key_data.subject),
                ),
            )
            .await?;

        Ok(())
    }
    pub fn new(
        vault: Arc<dyn IdentitiesVault>,
        purpose_keys: Arc<PurposeKeys>,
        identities_repository: Arc<dyn IdentitiesRepository>,
    ) -> Self {
        Self {
            vault,
            purpose_keys,
            identities_repository,
        }
    }
    pub fn vault(&self) -> Arc<dyn IdentitiesVault> {
        self.vault.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::identities::identities;
    use super::super::super::models::SchemaId;
    use super::*;
    use ockam_core::compat::collections::BTreeMap;

    #[tokio::test]
    async fn test_issue_credential() -> Result<()> {
        let identities = identities();
        let creation = identities.identities_creation();

        let issuer = creation.create_identity().await?;
        let subject = creation.create_identity().await?;

        let credentials_key = identities
            .purpose_keys()
            .create_purpose_key(issuer.identifier(), Purpose::Credentials)
            .await?;

        let credentials = identities.credentials();

        let mut map: BTreeMap<Vec<u8>, Vec<u8>> = Default::default();
        map.insert(b"key".to_vec(), b"value".to_vec());
        let subject_attributes = Attributes {
            schema: SchemaId(1),
            map,
        };

        let _credential = credentials
            .issue_credential(
                subject.identifier(),
                &credentials_key,
                subject_attributes,
                Duration::from_secs(60),
            )
            .await?;

        Ok(())
    }
}
