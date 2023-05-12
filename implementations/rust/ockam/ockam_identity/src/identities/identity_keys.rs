use crate::alloc::string::ToString;
use crate::identities::IdentitiesVault;
use crate::identity::IdentityChange::{CreateKey, RotateKey};
use crate::identity::IdentityError::InvalidInternalState;
use crate::identity::{
    ChangeIdentifier, CreateKeyChangeData, Identity, IdentityChangeConstants,
    IdentityChangeHistory, IdentityError, IdentitySignedChange, KeyAttributes, RotateKeyChangeData,
    Signature, SignatureType,
};
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::{Encodable, Result};
use ockam_vault::{KeyId, SecretAttributes, Vault};

/// This module supports the key operations related to identities
pub struct IdentitiesKeys {
    vault: Arc<dyn IdentitiesVault>,
}

impl IdentitiesKeys {
    pub(crate) async fn create_initial_key(
        &self,
        key_id: Option<&KeyId>,
        key_attribs: KeyAttributes,
    ) -> Result<IdentityChangeHistory> {
        let initial_change_id = self.make_change_identifier().await?;
        let create_key_change = self
            .make_create_key_change_static(key_id, initial_change_id, key_attribs.clone(), None)
            .await?;
        let change_history = IdentityChangeHistory::new(create_key_change);

        // Sanity checks
        change_history.check_entire_consistency()?;
        self.verify_all_existing_changes(&change_history).await?;

        Ok(change_history)
    }

    /// Initial `ChangeIdentifier` that is used as a previous_identifier of the first change
    async fn make_change_identifier(&self) -> Result<ChangeIdentifier> {
        let hash = Vault::sha256(IdentityChangeConstants::INITIAL_CHANGE);
        Ok(ChangeIdentifier::from_hash(hash))
    }
}

/// Public  functions
impl IdentitiesKeys {
    /// Create a new identities keys module
    pub fn new(vault: Arc<dyn IdentitiesVault>) -> Self {
        Self { vault }
    }

    /// Create a new identities keys module with an in-memory vault
    /// Sign some binary data with the signing key of an identity
    pub async fn create_signature(
        &self,
        identity: &Identity,
        data: &[u8],
        key_label: Option<&str>,
    ) -> Result<ockam_vault::Signature> {
        let secret = self.get_secret_key(identity, key_label).await?;
        self.vault.sign(&secret, data).await
    }

    /// Verify the signature of a piece of data
    pub async fn verify_signature(
        &self,
        identity: &Identity,
        signature: &ockam_vault::Signature,
        data: &[u8],
        key_label: Option<&str>,
    ) -> Result<bool> {
        let public_key = identity.get_public_key(key_label)?;
        self.vault.verify(&public_key, data, signature).await
    }

    /// Generate and add a new key to this `Identity` with a given `label`
    pub async fn create_key(&self, identity: &mut Identity, label: String) -> Result<()> {
        let key_attribs = KeyAttributes::default_with_label(label);
        let change = self
            .make_create_key_change(identity, None, key_attribs)
            .await?;
        identity.add_change(change)
    }

    /// Rotate an existing key with a given label
    pub async fn rotate_key(&self, identity: &mut Identity, label: &str) -> Result<()> {
        let change = self
            .make_rotate_key_change(
                identity,
                KeyAttributes::default_with_label(label.to_string()),
            )
            .await?;

        identity.add_change(change)
    }

    /// Add a new key to this `Identity` with a given `label`
    pub async fn add_key(
        &self,
        identity: &mut Identity,
        label: String,
        secret: &KeyId,
    ) -> Result<()> {
        let secret_attributes = self.vault.get_secret_attributes(secret).await?;
        let key_attribs = KeyAttributes::new(label, secret_attributes);

        let change = self
            .make_create_key_change(identity, Some(secret), key_attribs)
            .await?;

        identity.add_change(change)
    }

    /// Verify all changes present in current `IdentityChangeHistory`
    pub(crate) async fn verify_all_existing_changes(
        &self,
        identity_changes: &IdentityChangeHistory,
    ) -> Result<()> {
        for i in 0..identity_changes.as_ref().len() {
            let existing_changes = &identity_changes.as_ref()[..i];
            let new_change = &identity_changes.as_ref()[i];
            self.verify_change(existing_changes, new_change).await?
        }
        Ok(())
    }

    /// Return the secret key of an identity
    pub async fn get_secret_key(
        &self,
        identity: &Identity,
        key_label: Option<&str>,
    ) -> Result<KeyId> {
        let key = match key_label {
            Some(label) => self.get_labelled_key(identity, label).await?,
            None => self.get_root_secret_key(identity).await?,
        };
        Ok(key)
    }

    /// Rotate this `Identity` root key
    pub async fn rotate_root_key(&self, identity: &mut Identity) -> Result<()> {
        let change = self
            .make_rotate_key_change(
                identity,
                KeyAttributes::default_with_label(IdentityChangeConstants::ROOT_LABEL.to_string()),
            )
            .await?;

        identity.add_change(change)
    }

    /// Creates a signed static key to use for 'xx' key exchange
    pub async fn create_signed_static_key(
        &self,
        identity: &Identity,
    ) -> Result<(KeyId, ockam_vault::Signature)> {
        let static_key_id = self
            .vault
            .create_ephemeral_secret(SecretAttributes::X25519)
            .await?;

        let public_static_key = self.vault.get_public_key(&static_key_id).await?;

        let signature = self
            .create_signature(identity, public_static_key.data(), None)
            .await?;

        Ok((static_key_id, signature))
    }
}

/// Private  functions
impl IdentitiesKeys {
    /// Create a new key
    async fn make_create_key_change(
        &self,
        identity: &Identity,
        secret: Option<&KeyId>,
        key_attributes: KeyAttributes,
    ) -> Result<IdentitySignedChange> {
        let change_history = identity.change_history();
        // Creating key after it was revoked is forbidden
        if IdentityChangeHistory::find_last_key_change(
            change_history.as_ref(),
            key_attributes.label(),
        )
        .is_ok()
        {
            return Err(InvalidInternalState.into());
        }

        let prev_id = match change_history.get_last_change_id() {
            Ok(prev_id) => prev_id,
            Err(_) => self.make_change_identifier().await?,
        };

        let root_secret = self.get_root_secret_key(identity).await?;
        let root_key = Some(&root_secret);

        self.make_create_key_change_static(secret, prev_id, key_attributes, root_key)
            .await
    }

    /// Create a new key
    async fn make_create_key_change_static(
        &self,
        secret: Option<&KeyId>,
        prev_id: ChangeIdentifier,
        key_attributes: KeyAttributes,
        root_key: Option<&KeyId>,
    ) -> Result<IdentitySignedChange> {
        let secret_key = self.generate_key_if_needed(secret, &key_attributes).await?;
        let public_key = self.vault.get_public_key(&secret_key).await?;

        let data = CreateKeyChangeData::new(prev_id, key_attributes, public_key);

        let change_block = CreateKey(data);
        let change_block_binary = change_block
            .encode()
            .map_err(|_| IdentityError::BareError)?;

        let change_id = Vault::sha256(&change_block_binary);
        let change_id = ChangeIdentifier::from_hash(change_id);

        let self_signature = self.vault.sign(&secret_key, change_id.as_ref()).await?;
        let self_signature = Signature::new(SignatureType::SelfSign, self_signature);

        let mut signatures = vec![self_signature];

        // If we have root_key passed we should sign using it
        // If there is no root_key - we're creating new identity, so we just generated root_key
        if let Some(root_key) = root_key {
            let root_signature = self.vault.sign(root_key, change_id.as_ref()).await?;
            let root_signature = Signature::new(SignatureType::RootSign, root_signature);

            signatures.push(root_signature);
        }

        let signed_change = IdentitySignedChange::new(change_id, change_block, signatures);

        Ok(signed_change)
    }

    async fn generate_key_if_needed(
        &self,
        secret: Option<&KeyId>,
        key_attributes: &KeyAttributes,
    ) -> Result<KeyId> {
        if let Some(s) = secret {
            Ok(s.clone())
        } else {
            self.vault
                .create_persistent_secret(key_attributes.secret_attributes())
                .await
        }
    }

    async fn get_labelled_key(&self, identity: &Identity, label: &str) -> Result<KeyId> {
        let change =
            IdentityChangeHistory::find_last_key_change(identity.change_history().as_ref(), label)?
                .clone();
        self.get_secret_key_from_change(&change).await
    }

    /// Rotate key change
    async fn make_rotate_key_change(
        &self,
        identity: &mut Identity,
        key_attributes: KeyAttributes,
    ) -> Result<IdentitySignedChange> {
        let prev_change_id = identity.change_history.get_last_change_id()?;

        let last_change_in_chain = IdentityChangeHistory::find_last_key_change(
            identity.change_history.as_ref(),
            key_attributes.label(),
        )?
        .clone();

        let last_key_in_chain = self
            .get_secret_key_from_change(&last_change_in_chain)
            .await?;

        let secret_attributes = key_attributes.secret_attributes();

        let secret_key = self
            .vault
            .create_persistent_secret(secret_attributes)
            .await?;
        let public_key = self.vault.get_public_key(&secret_key).await?;

        let data = RotateKeyChangeData::new(prev_change_id, key_attributes, public_key);

        let change_block = RotateKey(data);
        let change_block_binary = change_block
            .encode()
            .map_err(|_| IdentityError::BareError)?;

        let change_id = Vault::sha256(&change_block_binary);
        let change_id = ChangeIdentifier::from_hash(change_id);

        let self_signature = self.vault.sign(&secret_key, change_id.as_ref()).await?;
        let self_signature = Signature::new(SignatureType::SelfSign, self_signature);

        let root_key = self.get_root_secret_key(identity).await?;

        let root_signature = self.vault.sign(&root_key, change_id.as_ref()).await?;
        let root_signature = Signature::new(SignatureType::RootSign, root_signature);

        let prev_signature = self
            .vault
            .sign(&last_key_in_chain, change_id.as_ref())
            .await?;
        let prev_signature = Signature::new(SignatureType::PrevSign, prev_signature);

        let signed_change = IdentitySignedChange::new(
            change_id,
            change_block,
            vec![self_signature, root_signature, prev_signature],
        );

        Ok(signed_change)
    }

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    async fn get_root_secret_key(&self, identity: &Identity) -> Result<KeyId> {
        self.get_labelled_key(identity, IdentityChangeConstants::ROOT_LABEL)
            .await
    }

    async fn get_secret_key_from_change(&self, change: &IdentitySignedChange) -> Result<KeyId> {
        let public_key = change.change().public_key()?;
        self.vault.get_key_id(&public_key).await
    }

    /// Verify all changes present in current `IdentityChangeHistory`
    pub async fn verify_changes(&self, identity: &Identity) -> Result<()> {
        self.verify_all_existing_changes(&identity.change_history())
            .await
    }

    /// WARNING: This function assumes all existing changes in chain are verified.
    /// WARNING: Correctness of changes sequence is not verified here.
    async fn verify_change(
        &self,
        existing_changes: &[IdentitySignedChange],
        new_change: &IdentitySignedChange,
    ) -> Result<()> {
        let change_binary = new_change
            .change()
            .encode()
            .map_err(|_| IdentityError::BareError)?;

        let change_id = Vault::sha256(&change_binary);
        let change_id = ChangeIdentifier::from_hash(change_id);

        if &change_id != new_change.identifier() {
            return Err(IdentityError::IdentityVerificationFailed.into()); // ChangeIdDoesNotMatch
        }

        struct SignaturesCheck {
            self_sign: u8,
            prev_sign: u8,
            root_sign: u8,
        }

        let mut signatures_check = match new_change.change() {
            CreateKey(_) => {
                // Should have self signature and root signature
                // There is no Root signature for the very first change
                let root_sign = u8::from(!existing_changes.is_empty());

                SignaturesCheck {
                    self_sign: 1,
                    prev_sign: 0,
                    root_sign,
                }
            }
            RotateKey(_) => {
                // Should have self signature, root signature, and previous key signature
                SignaturesCheck {
                    self_sign: 1,
                    prev_sign: 1,
                    root_sign: 1,
                }
            }
        };

        for signature in new_change.signatures() {
            let counter;
            let public_key = match signature.stype() {
                SignatureType::RootSign => {
                    if existing_changes.is_empty() {
                        return Err(IdentityError::IdentityVerificationFailed.into());
                    }

                    counter = &mut signatures_check.root_sign;
                    IdentityChangeHistory::get_current_root_public_key(existing_changes)?
                }
                SignatureType::SelfSign => {
                    counter = &mut signatures_check.self_sign;
                    new_change.change().public_key()?
                }
                SignatureType::PrevSign => {
                    counter = &mut signatures_check.prev_sign;
                    IdentityChangeHistory::get_public_key_static(
                        existing_changes,
                        new_change.change().label(),
                    )?
                }
            };

            if *counter == 0 {
                return Err(IdentityError::IdentityVerificationFailed.into());
            }

            if !self
                .vault
                .verify(&public_key, change_id.as_ref(), signature.data())
                .await?
            {
                return Err(IdentityError::IdentityVerificationFailed.into());
            }

            *counter -= 1;
        }

        if signatures_check.prev_sign == 0
            && signatures_check.root_sign == 0
            && signatures_check.self_sign == 0
        {
            Ok(())
        } else {
            Err(IdentityError::IdentityVerificationFailed.into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::identities;
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
        let mut identity = identities.identities_creation().create_identity().await?;

        identity_keys.verify_changes(&identity).await?;
        let secret1 = identity_keys.get_root_secret_key(&identity).await?;
        let public1 = identity.get_root_public_key()?;

        identity_keys
            .create_key(&mut identity, "Truck management".to_string())
            .await?;
        identity_keys.verify_changes(&identity).await?;

        let secret2 = identity_keys
            .get_labelled_key(&identity, "Truck management")
            .await?;
        let public2 = identity.get_public_key(Some("Truck management"))?;

        if secret1 == secret2 {
            return test_error("secret did not change after create_key");
        }

        if public1 == public2 {
            return test_error("public did not change after create_key");
        }

        identity_keys.rotate_root_key(&mut identity).await?;
        identity_keys.verify_changes(&identity).await?;

        let secret3 = identity_keys.get_root_secret_key(&identity).await?;
        let public3 = identity.get_root_public_key()?;

        identity_keys.rotate_root_key(&mut identity).await?;
        identity_keys.verify_changes(&identity).await?;

        if secret1 == secret3 {
            return test_error("secret did not change after rotate_key");
        }

        if public1 == public3 {
            return test_error("public did not change after rotate_key");
        }

        ctx.stop().await
    }
}
