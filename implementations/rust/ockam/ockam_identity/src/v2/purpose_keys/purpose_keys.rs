use super::super::models::{
    Ed25519Signature, PurposeKeyAttestation, PurposeKeyAttestationData,
    PurposeKeyAttestationSignature, PurposePublicKey, VersionedData,
};
use super::super::utils::now;
use super::super::{IdentitiesKeys, IdentitiesVault, Identity, Purpose, PurposeKey};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{SecretAttributes, Vault};

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct PurposeKeys {
    pub(crate) vault: Arc<dyn IdentitiesVault>,
    pub(crate) identity_keys: Arc<IdentitiesKeys>,
}

impl PurposeKeys {
    /// Return the identities vault
    pub fn vault(&self) -> Arc<dyn IdentitiesVault> {
        self.vault.clone()
    }
}

impl PurposeKeys {
    /// Create a new identities module
    pub(crate) fn new(vault: Arc<dyn IdentitiesVault>, identity_keys: Arc<IdentitiesKeys>) -> Self {
        Self {
            vault,
            identity_keys,
        }
    }
}

impl PurposeKeys {
    pub async fn create_purpose_key(
        &self,
        identity: &Identity,
        purpose: Purpose,
    ) -> Result<PurposeKey> {
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
                PurposePublicKey::SecureChannelAuthenticationKey(public_key.try_into().unwrap())
            }
            Purpose::Credentials => {
                PurposePublicKey::CredentialSigningKey(public_key.try_into().unwrap())
            }
        };

        let created_at = now()?;
        let expires_at = now()?; // FIXME

        let purpose_key_attestation_data = PurposeKeyAttestationData {
            subject: identity.identifier().clone(),
            subject_latest_change_hash: identity.latest_change_hash()?.clone(),
            public_key,
            created_at,
            expires_at,
        };

        let purpose_key_attestation_data = minicbor::to_vec(&purpose_key_attestation_data)?;

        let versioned_data = VersionedData {
            version: 1,
            data: purpose_key_attestation_data,
        };
        let versioned_data = minicbor::to_vec(&versioned_data)?;

        let hash = Vault::sha256(&versioned_data);

        let signing_key = self.identity_keys.get_secret_key(identity).await?;
        let signature = self.vault.sign(&signing_key, &hash).await?;
        let signature = Ed25519Signature(signature.as_ref().try_into().unwrap()); // FIXME
        let signature = PurposeKeyAttestationSignature::Ed25519Signature(signature);

        let attestation = PurposeKeyAttestation {
            data: versioned_data,
            signature,
        };

        Ok(PurposeKey::new(secret_key, purpose, attestation))
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{identities, Purpose};

    #[tokio::test]
    async fn create_purpose_keys() {
        let identities = identities();
        let identities_creation = identities.identities_creation();
        let purpose_keys = identities.purpose_keys();

        let identity = identities_creation.create_identity().await.unwrap();
        let _credentials_key = purpose_keys
            .create_purpose_key(&identity, Purpose::Credentials)
            .await
            .unwrap();
        let _secure_channel_key = purpose_keys
            .create_purpose_key(&identity, Purpose::SecureChannel)
            .await
            .unwrap();
    }
}
