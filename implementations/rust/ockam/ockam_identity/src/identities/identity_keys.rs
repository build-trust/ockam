use super::super::identity::Identity;
use super::super::models::{
    Change, ChangeData, ChangeHash, ChangeHistory, ChangeSignature, Ed25519PublicKey,
    Ed25519Signature, PrimaryPublicKey, VersionedData,
};
use super::super::utils::{add_seconds, now};
use super::super::IdentityError;

use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{KeyId, SecretAttributes, SigningVault, VerifyingVault};

/// This module supports the key operations related to identities
pub struct IdentitiesKeys {
    signing_vault: Arc<dyn SigningVault>,
    verifying_vault: Arc<dyn VerifyingVault>,
}

impl IdentitiesKeys {
    pub(crate) async fn create_initial_key(&self, key_id: Option<&KeyId>) -> Result<Identity> {
        let change = self.make_change(key_id, None).await?;
        let change_history = ChangeHistory(vec![change]);

        let identity = Identity::import_from_change_history(
            None,
            change_history,
            self.verifying_vault.clone(),
        )
        .await?;

        Ok(identity)
    }
}

/// Public functions
impl IdentitiesKeys {
    /// Create a new identities keys module
    pub fn new(
        signing_vault: Arc<dyn SigningVault>,
        verifying_vault: Arc<dyn VerifyingVault>,
    ) -> Self {
        Self {
            signing_vault,
            verifying_vault,
        }
    }

    /// Rotate an existing key with a given label
    pub async fn rotate_key(&self, identity: Identity) -> Result<Identity> {
        let last_change = match identity.changes().last() {
            Some(last_change) => last_change,
            None => return Err(IdentityError::EmptyIdentity.into()),
        };

        // TODO: Delete the previous key from the Vault
        let last_secret_key = self.get_secret_key(&identity).await?;

        let change = self
            .make_change(
                None,
                Some((last_change.change_hash().clone(), last_secret_key)),
            )
            .await?;

        identity
            .add_change(change, self.verifying_vault.clone())
            .await
    }

    /// Return the secret key of an identity
    pub async fn get_secret_key(&self, identity: &Identity) -> Result<KeyId> {
        if let Some(last_change) = identity.changes().last() {
            self.signing_vault
                .get_key_id(last_change.primary_public_key())
                .await
        } else {
            Err(IdentityError::EmptyIdentity.into())
        }
    }
}

/// Private  functions
impl IdentitiesKeys {
    /// Create a new key
    async fn make_change(
        &self,
        secret: Option<&KeyId>,
        previous: Option<(ChangeHash, KeyId)>,
    ) -> Result<Change> {
        let secret_key = self.generate_key_if_needed(secret).await?;
        let public_key = self.signing_vault.get_public_key(&secret_key).await?;

        let public_key = Ed25519PublicKey(public_key.data().try_into().unwrap()); // FIXME

        let created_at = now()?;
        let ten_years = 10 * 365 * 24 * 60 * 60; // TODO: Allow to customize
        let expires_at = add_seconds(&created_at, ten_years);

        let change_data = ChangeData {
            previous_change: previous.as_ref().map(|x| x.0.clone()),
            primary_public_key: PrimaryPublicKey::Ed25519PublicKey(public_key),
            revoke_all_purpose_keys: false, // TODO: Allow to choose
            created_at,
            expires_at,
        };

        let change_data = minicbor::to_vec(&change_data)?;

        let versioned_data = VersionedData {
            version: 1,
            data: change_data,
        };

        let versioned_data = minicbor::to_vec(&versioned_data)?;

        let hash = self.verifying_vault.sha256(&versioned_data).await?;

        let self_signature = self.signing_vault.sign(&secret_key, hash.as_ref()).await?;
        let self_signature = Ed25519Signature(self_signature.as_ref().try_into().unwrap()); // FIXME
        let self_signature = ChangeSignature::Ed25519Signature(self_signature);

        // If we have previous_key passed we should sign using it
        // If there is no previous_key - we're creating new identity, so we just generated the key
        let previous_signature = match previous.map(|x| x.1) {
            Some(previous_key) => {
                let previous_signature = self
                    .signing_vault
                    .sign(&previous_key, hash.as_ref())
                    .await?;
                let previous_signature =
                    Ed25519Signature(previous_signature.as_ref().try_into().unwrap()); // FIXME
                let previous_signature = ChangeSignature::Ed25519Signature(previous_signature);

                Some(previous_signature)
            }
            None => None,
        };

        let change = Change {
            data: versioned_data,
            signature: self_signature,
            previous_signature,
        };

        Ok(change)
    }

    async fn generate_key_if_needed(&self, secret: Option<&KeyId>) -> Result<KeyId> {
        if let Some(s) = secret {
            Ok(s.clone())
        } else {
            self.signing_vault
                .generate_key(SecretAttributes::Ed25519 /* FIXME */)
                .await
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::super::models::Identifier;
    use super::super::identities;
    use super::*;
    use core::str::FromStr;
    use ockam_core::errcode::{Kind, Origin};
    use ockam_core::Error;
    use ockam_node::Context;

    fn test_error<S: Into<String>>(error: S) -> Result<()> {
        Err(Error::new_without_cause(Origin::Identity, Kind::Unknown).context("msg", error.into()))
    }

    #[ockam_macros::test]
    async fn test_basic_identity_key_ops(ctx: &mut Context) -> Result<()> {
        let identities = identities();
        let identity_keys = identities.identities_keys();
        let identity = identities.identities_creation().create_identity().await?;

        // Identifier should not match
        let res = Identity::import_from_change_history(
            Some(&Identifier::from_str("Iabababababababababababababababababababab").unwrap()),
            identity.change_history().clone(),
            identities.vault().verifying_vault,
        )
        .await;
        assert!(res.is_err());

        // Check if verification succeeds
        let _ = Identity::import_from_change_history(
            Some(identity.identifier()),
            identity.change_history().clone(),
            identities.vault().verifying_vault,
        )
        .await?;

        let secret1 = identity_keys.get_secret_key(&identity).await?;
        let public1 = identity.get_public_key()?;

        let identity = identity_keys.rotate_key(identity).await?;

        // Check if verification succeeds
        let _ = Identity::import_from_change_history(
            Some(identity.identifier()),
            identity.change_history().clone(),
            identities.vault().verifying_vault,
        )
        .await?;

        let secret2 = identity_keys.get_secret_key(&identity).await?;
        let public2 = identity.get_public_key()?;

        if secret1 == secret2 {
            return test_error("secret did not change after rotate_key");
        }

        if public1 == public2 {
            return test_error("public did not change after rotate_key");
        }

        ctx.stop().await
    }
}
