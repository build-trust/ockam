use super::super::identities::AttributesEntry;
use super::super::models::{
    Attributes, Credential, CredentialAndPurposeKey, CredentialData, CredentialSignature,
    Ed25519Signature, Identifier, PurposeKeyAttestationData, PurposePublicKey, VersionedData,
};
use super::super::utils::{add_seconds, now};
use super::super::{IdentitiesRepository, Identity, IdentityError, Purpose, PurposeKeys};

use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::{SecretType, Signature, SigningVault, VerifyingVault};

/// Structure with both [`CredentialData`] and [`PurposeKeyAttestationData`] that we get
/// after parsing and verifying corresponding [`Credential`] and [`super::super::models::PurposeKeyAttestation`]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialAndPurposeKeyData {
    /// [`CredentialData`]
    pub credential_data: CredentialData,
    /// [`PurposeKeyAttestationData`]
    pub purpose_key_data: PurposeKeyAttestationData,
}

/// Service for managing [`Credential`]s
pub struct Credentials {
    signing_vault: Arc<dyn SigningVault>,
    verifying_vault: Arc<dyn VerifyingVault>,
    purpose_keys: Arc<PurposeKeys>,
    identities_repository: Arc<dyn IdentitiesRepository>,
}

impl Credentials {
    ///Constructor
    pub fn new(
        signing_vault: Arc<dyn SigningVault>,
        verifying_vault: Arc<dyn VerifyingVault>,
        purpose_keys: Arc<PurposeKeys>,
        identities_repository: Arc<dyn IdentitiesRepository>,
    ) -> Self {
        Self {
            signing_vault,
            verifying_vault,
            purpose_keys,
            identities_repository,
        }
    }

    /// [`PurposeKeys`]
    pub fn purpose_keys(&self) -> Arc<PurposeKeys> {
        self.purpose_keys.clone()
    }

    /// [`IdentitiesRepository`]
    pub fn identities_repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.identities_repository.clone()
    }
}

impl Credentials {
    /// Verify a [`Credential`]
    pub async fn verify_credential(
        &self,
        expected_subject: Option<&Identifier>,
        authorities: &[Identifier],
        credential_and_purpose_key: &CredentialAndPurposeKey,
    ) -> Result<CredentialAndPurposeKeyData> {
        let purpose_key_data = self
            .purpose_keys
            .verify_purpose_key_attestation(&credential_and_purpose_key.purpose_key_attestation)
            .await?;

        if !authorities.contains(&purpose_key_data.subject) {
            return Err(IdentityError::UnknownAuthority.into());
        }

        let public_key = match purpose_key_data.public_key.clone() {
            PurposePublicKey::SecureChannelStaticKey(_) => {
                return Err(IdentityError::InvalidKeyType.into())
            }

            PurposePublicKey::CredentialSigningKey(public_key) => public_key,
        };

        let public_key = public_key.into();

        let versioned_data_hash = self
            .verifying_vault
            .sha256(&credential_and_purpose_key.credential.data)
            .await?;

        let signature = match &credential_and_purpose_key.credential.signature {
            CredentialSignature::Ed25519Signature(signature) => {
                Signature::new(signature.0.to_vec())
            }
            CredentialSignature::P256ECDSASignature(_) => {
                return Err(IdentityError::InvalidKeyType.into())
            }
        };

        if !self
            .verifying_vault
            .verify(&public_key, &versioned_data_hash, &signature)
            .await?
        {
            return Err(IdentityError::CredentialVerificationFailed.into());
        }

        let versioned_data: VersionedData =
            minicbor::decode(&credential_and_purpose_key.credential.data)?;
        if versioned_data.version != 1 {
            return Err(IdentityError::UnknownCredentialVersion.into());
        }

        let credential_data: CredentialData = minicbor::decode(&versioned_data.data)?;

        if credential_data.subject.is_none() && credential_data.subject_latest_change_hash.is_none()
        {
            return Err(IdentityError::CredentialVerificationFailed.into());
        }

        if credential_data.subject.as_ref() != expected_subject {
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

        Ok(CredentialAndPurposeKeyData {
            credential_data,
            purpose_key_data,
        })
    }

    /// Issue a [`Credential`]
    pub async fn issue_credential(
        &self,
        issuer: &Identifier,
        subject: &Identifier,
        subject_attributes: Attributes,
        ttl: Duration,
    ) -> Result<CredentialAndPurposeKey> {
        let issuer_purpose_key = self
            .purpose_keys
            .repository()
            .get_purpose_key(issuer, Purpose::Credentials)
            .await?;

        let issuer_purpose_key = self
            .purpose_keys
            .import_purpose_key(&issuer_purpose_key)
            .await?;

        let subject_change_history = self.identities_repository.get_identity(subject).await?;
        let subject_identity = Identity::import_from_change_history(
            Some(subject),
            subject_change_history,
            self.verifying_vault.clone(),
        )
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

        let versioned_data_hash = self.verifying_vault.sha256(&versioned_data).await?;

        if issuer_purpose_key.purpose() != Purpose::Credentials {
            return Err(IdentityError::InvalidKeyType.into());
        }

        if issuer_purpose_key.stype() != SecretType::Ed25519 {
            return Err(IdentityError::InvalidKeyType.into());
        }

        let signature = self
            .signing_vault
            .sign(issuer_purpose_key.key_id(), &versioned_data_hash)
            .await?;
        let signature: Vec<u8> = signature.into();
        let signature = Ed25519Signature(signature.try_into().unwrap());
        let signature = CredentialSignature::Ed25519Signature(signature);

        let credential = Credential {
            data: versioned_data,
            signature,
        };

        let res = CredentialAndPurposeKey {
            credential,
            purpose_key_attestation: issuer_purpose_key.attestation().clone(),
        };

        Ok(res)
    }

    /// Receive someone's [`Credential`]: verify and put attributes from it to the storage
    pub async fn receive_presented_credential(
        &self,
        subject: &Identifier,
        authorities: &[Identifier],
        credential_and_purpose_key_attestation: &CredentialAndPurposeKey,
    ) -> Result<()> {
        let credential_data = self
            .verify_credential(
                Some(subject),
                authorities,
                credential_and_purpose_key_attestation,
            )
            .await?;

        self.identities_repository
            .put_attributes(
                subject,
                AttributesEntry::new(
                    credential_data.credential_data.subject_attributes.map,
                    now()?,
                    Some(credential_data.credential_data.expires_at),
                    Some(credential_data.purpose_key_data.subject),
                ),
            )
            .await?;

        Ok(())
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

        let _credentials_key = identities
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

        let credential = credentials
            .issue_credential(
                issuer.identifier(),
                subject.identifier(),
                subject_attributes,
                Duration::from_secs(60),
            )
            .await?;

        let _res = credentials
            .verify_credential(
                Some(subject.identifier()),
                &[issuer.identifier().clone()],
                &credential,
            )
            .await?;

        Ok(())
    }
}
