use crate::identity::Identity;
use crate::models::{Change, ChangeData, ChangeHash, ChangeHistory, VersionedData};
use crate::{IdentityError, IdentityOptions};

use ockam_core::compat::sync::Arc;
use ockam_core::Result;

use ockam_vault::{SigningSecretKeyHandle, VaultForSigning, VaultForVerifyingSignatures};
use tracing::error;

/// This module supports the key operations related to identities
pub struct IdentitiesKeys {
    identity_vault: Arc<dyn VaultForSigning>,
    verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
}

impl IdentitiesKeys {
    pub(crate) async fn create_initial_key(&self, options: IdentityOptions) -> Result<Identity> {
        let change = self.make_change(options, None).await?;
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
        identity_vault: Arc<dyn VaultForSigning>,
        verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
    ) -> Self {
        Self {
            identity_vault,
            verifying_vault,
        }
    }

    /// Rotate the Identity Key
    pub async fn rotate_key_with_options(
        &self,
        identity: Identity,
        options: IdentityOptions,
    ) -> Result<Identity> {
        let last_change = match identity.changes().last() {
            Some(last_change) => last_change,
            None => return Err(IdentityError::EmptyIdentity.into()),
        };

        let last_secret_key = self.get_secret_key(&identity).await?;

        let change = self
            .make_change(
                options,
                Some((last_change.change_hash().clone(), last_secret_key.clone())),
            )
            .await?;

        let identity = identity
            .add_change(change, self.verifying_vault.clone())
            .await?;

        if self
            .identity_vault
            .delete_signing_secret_key(last_secret_key)
            .await
            .is_err()
        {
            error!(
                "Error deleting old Identity Key for {}",
                identity.identifier()
            );
        }

        Ok(identity)
    }

    /// Return the secret key of an identity
    pub async fn get_secret_key(&self, identity: &Identity) -> Result<SigningSecretKeyHandle> {
        if let Some(last_change) = identity.changes().last() {
            self.identity_vault
                .get_secret_key_handle(last_change.primary_public_key())
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
        identity_options: IdentityOptions,
        previous: Option<(ChangeHash, SigningSecretKeyHandle)>,
    ) -> Result<Change> {
        let secret_key = identity_options.signing_secret_key_handle;
        let public_key = self
            .identity_vault
            .get_verifying_public_key(&secret_key)
            .await?;

        let change_data = ChangeData {
            previous_change: previous.as_ref().map(|x| x.0.clone()),
            primary_public_key: public_key.into(),
            revoke_all_purpose_keys: identity_options.revoke_all_purpose_keys,
            created_at: identity_options.created_at,
            expires_at: identity_options.expires_at,
        };

        let change_data = minicbor::to_vec(&change_data)?;

        let versioned_data = VersionedData {
            version: 1,
            data: change_data,
        };

        let versioned_data = minicbor::to_vec(&versioned_data)?;

        let hash = self.verifying_vault.sha256(&versioned_data).await?;

        let self_signature = self.identity_vault.sign(&secret_key, &hash.0).await?;
        let self_signature = self_signature.into();

        // If we have previous_key passed we should sign using it
        // If there is no previous_key - we're creating new identity, so we just generated the key
        let previous_signature = match previous.map(|x| x.1) {
            Some(previous_key) => {
                let previous_signature = self.identity_vault.sign(&previous_key, &hash.0).await?;

                Some(previous_signature.into())
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
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::identities;
    use crate::models::Identifier;
    use crate::utils::now;
    use core::str::FromStr;
    use ockam_core::errcode::{Kind, Origin};
    use ockam_core::Error;
    use ockam_node::Context;
    use ockam_vault::SigningKeyType;

    fn test_error<S: Into<String>>(error: S) -> Result<()> {
        Err(Error::new_without_cause(Origin::Identity, Kind::Unknown).context("msg", error.into()))
    }

    #[ockam_macros::test]
    async fn test_basic_identity_key_ops(ctx: &mut Context) -> Result<()> {
        let identities = identities();
        let identities_keys = identities.identities_keys();

        let key1 = identities_keys
            .identity_vault
            .generate_signing_secret_key(SigningKeyType::EdDSACurve25519)
            .await?;

        let now = now()?;
        let created_at1 = now;
        let expires_at1 = created_at1 + 120.into();

        let options1 = IdentityOptions::new(key1.clone(), false, created_at1, expires_at1);
        let identity1 = identities_keys.create_initial_key(options1).await?;

        // Identifier should not match
        let res = Identity::import_from_change_history(
            Some(&Identifier::from_str("Iabababababababababababababababababababab").unwrap()),
            identity1.change_history().clone(),
            identities.vault().verifying_vault,
        )
        .await;
        assert!(res.is_err());

        // Check if verification succeeds
        let _ = Identity::import_from_change_history(
            Some(identity1.identifier()),
            identity1.change_history().clone(),
            identities.vault().verifying_vault,
        )
        .await?;

        let secret1 = identities_keys.get_secret_key(&identity1).await?;
        let public1 = identity1.get_latest_public_key()?;
        assert_eq!(secret1, key1);

        let key2 = identities_keys
            .identity_vault
            .generate_signing_secret_key(SigningKeyType::EdDSACurve25519)
            .await?;

        let created_at2 = now + 10.into();
        let expires_at2 = created_at2 + 120.into();
        let options2 = IdentityOptions::new(key2.clone(), false, created_at2, expires_at2);
        let identity2 = identities_keys
            .rotate_key_with_options(identity1, options2)
            .await?;

        // Identifier should not match
        let res = Identity::import_from_change_history(
            Some(&Identifier::from_str("Iabababababababababababababababababababab").unwrap()),
            identity2.change_history().clone(),
            identities.vault().verifying_vault,
        )
        .await;
        assert!(res.is_err());

        // Check if verification succeeds
        let _ = Identity::import_from_change_history(
            Some(identity2.identifier()),
            identity2.change_history().clone(),
            identities.vault().verifying_vault,
        )
        .await?;

        let secret2 = identities_keys.get_secret_key(&identity2).await?;
        let public2 = identity2.get_latest_public_key()?;
        assert_eq!(secret2, key2);

        if secret1 == secret2 {
            return test_error("secret did not change after rotate_key");
        }

        if public1 == public2 {
            return test_error("public did not change after rotate_key");
        }

        // Old key should be deleted
        assert!(identities
            .vault()
            .identity_vault
            .get_verifying_public_key(&secret1)
            .await
            .is_err());
        // New key should exist
        assert!(identities
            .vault()
            .identity_vault
            .get_verifying_public_key(&secret2)
            .await
            .is_ok());

        ctx.stop().await
    }
}
