use crate::alloc::string::ToString;
use crate::identities::IdentitiesVault;
use crate::identity::IdentityError::InvalidInternalState;
use crate::identity::{Identity, IdentityConstants, IdentityError};
use crate::models::{
    now, Change, ChangeData, ChangeHash, ChangeHistory, ChangeSignature, Ed25519PublicKey,
    Ed25519Signature, Identifier, PrimaryPublicKey, TimestampInSeconds, VersionedData,
};
use crate::verified_change::VerifiedChange;
use crate::ChangeHistoryBinary;
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::{Encodable, Result};
use ockam_vault::{KeyId, PublicKey, SecretAttributes, SecretType, Signature, Vault};

/// This module supports the key operations related to identities
pub struct IdentitiesKeys {
    vault: Arc<dyn IdentitiesVault>,
}

impl IdentitiesKeys {
    pub(crate) async fn create_initial_key(&self, key_id: Option<&KeyId>) -> Result<Identity> {
        let initial_change_hash = self.make_initial_change_hash().await;
        let change = self
            .make_change_static(key_id, initial_change_hash, None)
            .await?;
        let change_history = ChangeHistory(vec![change]);
        let change_history_binary = ChangeHistoryBinary(minicbor::to_vec(&change_history)?);

        let verified_changes = Self::check_entire_consistency(&change_history.0).await?;
        self.verify_all_existing_changes(&verified_changes, &change_history.0)
            .await?;

        let identifier = if let Some(first_change) = verified_changes.first() {
            first_change.change_hash().clone().into()
        } else {
            return Err(IdentityError::IdentityVerificationFailed.into());
        };
        let identity = Identity::new(identifier, verified_changes, change_history_binary);

        Ok(identity)
    }

    /// Initial `ChangeIdentifier` that is used as a previous_identifier of the first change
    async fn make_initial_change_hash(&self) -> ChangeHash {
        let hash = Vault::sha256(IdentityConstants::BUILD_TRUST);
        ChangeHash::new(hash)
    }

    /// Check consistency of changes that are being added
    pub async fn check_entire_consistency(changes: &[Change]) -> Result<Vec<VerifiedChange>> {
        let to_be_verified_changes = Self::check_consistency(&[], changes).await?;

        Ok(to_be_verified_changes)
    }
}

struct ChangeDetails {
    version: u8,
    change_hash: ChangeHash,
    change_data: ChangeData,
}

impl IdentitiesKeys {
    async fn get_change_details(change: &Change) -> Result<ChangeDetails> {
        let change_hash = Self::compute_change_hash(&change.data).await;
        let versioned_data: VersionedData = minicbor::decode(&change.data)?;
        let change_data: ChangeData = minicbor::decode(&versioned_data.data)?;

        Ok(ChangeDetails {
            version: versioned_data.version,
            change_hash,
            change_data,
        })
    }

    /// Check consistency of changes that are been added
    async fn check_consistency(
        existing_changes: &[Change],
        new_changes: &[Change],
    ) -> Result<Vec<VerifiedChange>> {
        let mut to_be_verified_changes = Vec::with_capacity(new_changes.len());

        let mut previous_change_details = match existing_changes.last() {
            Some(previous_change) => Some(Self::get_change_details(previous_change).await?),
            None => None,
        };

        for change in new_changes.iter() {
            let change_details = Self::get_change_details(change).await?;

            if let Some(previous_change_details) = previous_change_details {
                if previous_change_details.version > change_details.version {
                    // Version downgrade
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }

                if previous_change_details.change_hash != change_details.change_data.previous_change
                {
                    // Corrupted changes sequence
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }
            }

            to_be_verified_changes.push(VerifiedChange::new(
                change_details.change_hash.clone(),
                change_details.change_data.primary_public_key.clone().into(),
                change_details.change_data.revoke_all_purpose_keys,
            ));

            previous_change_details = Some(change_details);
        }

        Ok(to_be_verified_changes)
    }
}

/// Public functions
impl IdentitiesKeys {
    /// Create a new identities keys module
    pub fn new(vault: Arc<dyn IdentitiesVault>) -> Self {
        Self { vault }
    }

    // /// Create a new identities keys module with an in-memory vault
    // /// Sign some binary data with the signing key of an identity
    // pub async fn create_signature(
    //     &self,
    //     identity: &Identity,
    //     data: &[u8],
    //     key_label: Option<&str>,
    // ) -> Result<ockam_vault::Signature> {
    //     let secret = self.get_secret_key(identity, key_label).await?;
    //     self.vault.sign(&secret, data).await
    // }
    //
    // /// Verify the signature of a piece of data
    // pub async fn verify_signature(
    //     &self,
    //     identity: &Identity,
    //     signature: &ockam_vault::Signature,
    //     data: &[u8],
    //     key_label: Option<&str>,
    // ) -> Result<bool> {
    //     let public_key = identity.get_public_key(key_label)?;
    //     self.vault.verify(&public_key, data, signature).await
    // }
    //
    // /// Generate and add a new key to this `Identity` with a given `label`
    // pub async fn create_key(&self, identity: &mut Identity, label: String) -> Result<()> {
    //     let key_attribs = KeyAttributes::default_with_label(label);
    //     let change = self
    //         .make_create_key_change(identity, None, key_attribs)
    //         .await?;
    //     identity.add_change(change)
    // }
    //
    // /// Rotate an existing key with a given label
    // pub async fn rotate_key(&self, identity: &mut Identity, label: &str) -> Result<()> {
    //     let change = self
    //         .make_rotate_key_change(
    //             identity,
    //             KeyAttributes::default_with_label(label.to_string()),
    //         )
    //         .await?;
    //
    //     identity.add_change(change)
    // }
    //
    // /// Add a new key to this `Identity` with a given `label`
    // pub async fn add_key(
    //     &self,
    //     identity: &mut Identity,
    //     label: String,
    //     secret: &KeyId,
    // ) -> Result<()> {
    //     let secret_attributes = self.vault.get_secret_attributes(secret).await?;
    //     let key_attribs = KeyAttributes::new(label, secret_attributes);
    //
    //     let change = self
    //         .make_create_key_change(identity, Some(secret), key_attribs)
    //         .await?;
    //
    //     identity.add_change(change)
    // }
    //
    /// Verify all changes present in current `IdentityChangeHistory`
    pub(crate) async fn verify_all_existing_changes(
        &self,
        to_be_verified_changes: &[VerifiedChange],
        changes: &[Change],
    ) -> Result<()> {
        let len = to_be_verified_changes.len();
        if len != changes.len() {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        for i in 0..len {
            let existing_changes = &to_be_verified_changes[..i];
            let new_change = &changes[i];
            self.verify_change(existing_changes, new_change).await?
        }
        Ok(())
    }

    // /// Return the secret key of an identity
    // pub async fn get_secret_key(
    //     &self,
    //     identity: &Identity,
    //     key_label: Option<&str>,
    // ) -> Result<KeyId> {
    //     let key = match key_label {
    //         Some(label) => self.get_labelled_key(identity, label).await?,
    //         None => self.get_root_secret_key(identity).await?,
    //     };
    //     Ok(key)
    // }
    //
    // /// Rotate this `Identity` root key
    // pub async fn rotate_root_key(&self, identity: &mut Identity) -> Result<()> {
    //     let change = self
    //         .make_rotate_key_change(
    //             identity,
    //             KeyAttributes::default_with_label(IdentityConstants::ROOT_LABEL.to_string()),
    //         )
    //         .await?;
    //
    //     identity.add_change(change)
    // }
    //
    // /// Creates a signed static key to use for 'xx' key exchange
    // pub async fn create_signed_static_key(
    //     &self,
    //     identity: &Identity,
    // ) -> Result<(KeyId, ockam_vault::Signature)> {
    //     let static_key_id = self
    //         .vault
    //         .create_ephemeral_secret(SecretAttributes::X25519)
    //         .await?;
    //
    //     let public_static_key = self.vault.get_public_key(&static_key_id).await?;
    //
    //     let signature = self
    //         .create_signature(identity, public_static_key.data(), None)
    //         .await?;
    //
    //     Ok((static_key_id, signature))
    // }
}

/// Private  functions
impl IdentitiesKeys {
    // /// Create a new key
    // async fn make_create_key_change(
    //     &self,
    //     identity: &Identity,
    //     secret: Option<&KeyId>,
    //     key_attributes: KeyAttributes,
    // ) -> Result<IdentitySignedChange> {
    //     let change_history = identity.change_history();
    //     // Creating key after it was revoked is forbidden
    //     if IdentityChangeHistory::find_last_key_change(
    //         change_history.as_ref(),
    //         key_attributes.label(),
    //     )
    //     .is_ok()
    //     {
    //         return Err(InvalidInternalState.into());
    //     }
    //
    //     let prev_id = match change_history.get_last_change_id() {
    //         Ok(prev_id) => prev_id,
    //         Err(_) => self.make_change_identifier().await?,
    //     };
    //
    //     let root_secret = self.get_root_secret_key(identity).await?;
    //     let root_key = Some(&root_secret);
    //
    //     self.make_change_static(secret, prev_id, key_attributes, root_key)
    //         .await
    // }
    //

    async fn compute_change_hash(versioned_data: &[u8]) -> ChangeHash {
        let change_hash = Vault::sha256(&versioned_data);

        ChangeHash::new(change_hash)
    }

    /// Create a new key
    async fn make_change_static(
        &self,
        secret: Option<&KeyId>,
        previous_change: ChangeHash,
        previous_key: Option<&KeyId>,
    ) -> Result<Change> {
        let secret_key = self.generate_key_if_needed(secret).await?;
        let public_key = self.vault.get_public_key(&secret_key).await?;

        let public_key = Ed25519PublicKey(public_key.data().try_into().unwrap()); // FIXME

        let created_at = now()?;
        let expires_at = now()?; // FIXME

        let change_data = ChangeData {
            previous_change,
            primary_public_key: PrimaryPublicKey::Ed25519PublicKey(public_key),
            revoke_all_purpose_keys: false, // FIXME
            created_at,
            expires_at,
        };

        let change_data = minicbor::to_vec(&change_data)?;

        let versioned_data = VersionedData {
            version: 1,
            data: change_data,
        };

        let versioned_data = minicbor::to_vec(&versioned_data)?;

        let change_hash = Self::compute_change_hash(&versioned_data).await;

        let self_signature = self.vault.sign(&secret_key, change_hash.as_ref()).await?;
        let self_signature = Ed25519Signature(self_signature.as_ref().try_into().unwrap()); // FIXME
        let self_signature = ChangeSignature::Ed25519Signature(self_signature);

        // If we have previous_key passed we should sign using it
        // If there is no previous_key - we're creating new identity, so we just generated the key
        let previous_signature = match previous_key {
            Some(previous_key) => {
                let previous_signature =
                    self.vault.sign(previous_key, change_hash.as_ref()).await?;
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
            self.vault
                .create_persistent_secret(SecretAttributes::Ed25519 /* FIXME */)
                .await
        }
    }

    // async fn get_labelled_key(&self, identity: &Identity, label: &str) -> Result<KeyId> {
    //     let change =
    //         IdentityChangeHistory::find_last_key_change(identity.change_history().as_ref(), label)?
    //             .clone();
    //     self.get_secret_key_from_change(&change).await
    // }
    //
    // /// Rotate key change
    // async fn make_rotate_key_change(
    //     &self,
    //     identity: &mut Identity,
    //     key_attributes: KeyAttributes,
    // ) -> Result<IdentitySignedChange> {
    //     let prev_change_id = identity.change_history.get_last_change_id()?;
    //
    //     let last_change_in_chain = IdentityChangeHistory::find_last_key_change(
    //         identity.change_history.as_ref(),
    //         key_attributes.label(),
    //     )?
    //     .clone();
    //
    //     let last_key_in_chain = self
    //         .get_secret_key_from_change(&last_change_in_chain)
    //         .await?;
    //
    //     let secret_attributes = key_attributes.secret_attributes();
    //
    //     let secret_key = self
    //         .vault
    //         .create_persistent_secret(secret_attributes)
    //         .await?;
    //     let public_key = self.vault.get_public_key(&secret_key).await?;
    //
    //     let data = RotateKeyChangeData::new(prev_change_id, key_attributes, public_key);
    //
    //     let change_block = RotateKey(data);
    //     let change_block_binary = change_block
    //         .encode()
    //         .map_err(|_| IdentityError::BareError)?;
    //
    //     let change_id = Vault::sha256(&change_block_binary);
    //     let change_id = ChangeIdentifier::from_hash(change_id);
    //
    //     let self_signature = self.vault.sign(&secret_key, change_id.as_ref()).await?;
    //     let self_signature = Signature::new(SignatureType::SelfSign, self_signature);
    //
    //     let root_key = self.get_root_secret_key(identity).await?;
    //
    //     let root_signature = self.vault.sign(&root_key, change_id.as_ref()).await?;
    //     let root_signature = Signature::new(SignatureType::RootSign, root_signature);
    //
    //     let prev_signature = self
    //         .vault
    //         .sign(&last_key_in_chain, change_id.as_ref())
    //         .await?;
    //     let prev_signature = Signature::new(SignatureType::PrevSign, prev_signature);
    //
    //     let signed_change = IdentitySignedChange::new(
    //         change_id,
    //         change_block,
    //         vec![self_signature, root_signature, prev_signature],
    //     );
    //
    //     Ok(signed_change)
    // }
    //
    // /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    // async fn get_root_secret_key(&self, identity: &Identity) -> Result<KeyId> {
    //     self.get_labelled_key(identity, IdentityConstants::ROOT_LABEL)
    //         .await
    // }
    //
    // async fn get_secret_key_from_change(&self, change: &IdentitySignedChange) -> Result<KeyId> {
    //     let public_key = change.change().public_key()?;
    //     self.vault.get_key_id(&public_key).await
    // }
    //
    // /// Verify all changes present in current `IdentityChangeHistory`
    // pub async fn verify_changes(&self, identity: &Identity) -> Result<()> {
    //     self.verify_all_existing_changes(&identity.change_history())
    //         .await
    // }

    async fn verify_signature(
        &self,
        public_key: &PublicKey,
        change_hash: &ChangeHash,
        signature: &ChangeSignature,
    ) -> Result<bool> {
        let signature = match signature {
            ChangeSignature::Ed25519Signature(signature) => {
                if public_key.stype() != SecretType::Ed25519 {
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }
                Signature::new(signature.0.to_vec())
            }
            ChangeSignature::P256ECDSASignature(signature) => {
                if public_key.stype() != SecretType::NistP256 {
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }
                Signature::new(signature.0.to_vec())
            }
        };

        self.vault
            .verify(public_key, change_hash.as_ref(), &signature)
            .await
    }

    /// WARNING: This function assumes all existing changes in chain are verified.
    /// WARNING: Correctness of changes sequence is not verified here.
    async fn verify_change(
        &self,
        existing_changes: &[VerifiedChange],
        new_change: &Change,
    ) -> Result<()> {
        let change_hash = Self::compute_change_hash(&new_change.data).await;

        let versioned_data: VersionedData = minicbor::decode(&new_change.data)?;

        let change_data: ChangeData = minicbor::decode(&versioned_data.data)?;

        if let Some(last_verified_change) = existing_changes.last() {
            if let Some(previous_signature) = &new_change.previous_signature {
                if !self
                    .verify_signature(
                        last_verified_change.primary_public_key(),
                        &change_hash,
                        previous_signature,
                    )
                    .await?
                {
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }
            } else {
                return Err(IdentityError::IdentityVerificationFailed.into());
            }
        }

        if !self
            .verify_signature(
                &change_data.primary_public_key.clone().into(),
                &change_hash,
                &new_change.signature,
            )
            .await?
        {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        Ok(())
    }
}
