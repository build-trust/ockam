use super::super::models::{Change, ChangeData, ChangeHash, ChangeSignature, VersionedData};
use super::super::verified_change::VerifiedChange;
use super::super::{IdentitiesVault, Identity, IdentityError};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::{PublicKey, SecretType, Signature, Vault};

struct ChangeDetails {
    version: u8,
    change_hash: ChangeHash,
    change_data: ChangeData,
}

impl Identity {
    pub(crate) fn compute_change_hash_from_data(data: &[u8]) -> ChangeHash {
        let hash = Vault::sha256(data);
        Self::compute_change_hash_from_hash(hash)
    }

    pub(crate) fn compute_change_hash_from_hash(hash: [u8; 32]) -> ChangeHash {
        let change_hash = hash[0..20].try_into().unwrap();
        ChangeHash::new(change_hash)
    }

    /// Check consistency of changes that are being added
    pub async fn check_entire_consistency(changes: &[Change]) -> Result<Vec<VerifiedChange>> {
        let to_be_verified_changes = Self::check_consistency(&[], changes).await?;

        Ok(to_be_verified_changes)
    }

    fn get_change_details(change: &Change) -> Result<ChangeDetails> {
        let change_hash = Self::compute_change_hash_from_data(&change.data);
        let versioned_data: VersionedData = minicbor::decode(&change.data)?;

        if versioned_data.version != 1 {
            return Err(IdentityError::UnknownIdentityVersion.into());
        }

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
            Some(previous_change) => Some(Self::get_change_details(previous_change)?),
            None => None,
        };

        for change in new_changes.iter() {
            let change_details = Self::get_change_details(change)?;

            if let Some(previous_change_details) = previous_change_details {
                if previous_change_details.version > change_details.version {
                    // Version downgrade
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }

                if let Some(previous_change_hash) = &change_details.change_data.previous_change {
                    if &previous_change_details.change_hash != previous_change_hash {
                        // Corrupted changes sequence
                        return Err(IdentityError::IdentityVerificationFailed.into());
                    }
                }
            } else if change_details.change_data.previous_change.is_some() {
                // Should empty
                return Err(IdentityError::IdentityVerificationFailed.into());
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

    /// Verify all changes present in current `IdentityChangeHistory`
    pub(crate) async fn verify_all_existing_changes(
        to_be_verified_changes: &[VerifiedChange],
        changes: &[Change],
        vault: Arc<dyn IdentitiesVault>,
    ) -> Result<()> {
        let len = to_be_verified_changes.len();
        if len != changes.len() {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        for i in 0..len {
            let existing_changes = &to_be_verified_changes[..i];
            let new_change = &changes[i];
            Self::verify_change(existing_changes, new_change, vault.clone()).await?
        }
        Ok(())
    }

    async fn verify_change_signature(
        public_key: &PublicKey,
        hash: [u8; 32],
        signature: &ChangeSignature,
        vault: Arc<dyn IdentitiesVault>,
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

        vault.verify(public_key, &hash, &signature).await
    }

    /// WARNING: This function assumes all existing changes in chain are verified.
    /// WARNING: Correctness of changes sequence is not verified here.
    async fn verify_change(
        existing_changes: &[VerifiedChange],
        new_change: &Change,
        vault: Arc<dyn IdentitiesVault>,
    ) -> Result<()> {
        let hash = Vault::sha256(&new_change.data);

        let versioned_data: VersionedData = minicbor::decode(&new_change.data)?;

        if versioned_data.version != 1 {
            return Err(IdentityError::UnknownIdentityVersion.into());
        }

        let change_data: ChangeData = minicbor::decode(&versioned_data.data)?;

        if let Some(last_verified_change) = existing_changes.last() {
            if let Some(previous_signature) = &new_change.previous_signature {
                if !Self::verify_change_signature(
                    last_verified_change.primary_public_key(),
                    hash,
                    previous_signature,
                    vault.clone(),
                )
                .await?
                {
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }
            } else {
                return Err(IdentityError::IdentityVerificationFailed.into());
            }
        }

        if !Self::verify_change_signature(
            &change_data.primary_public_key.clone().into(),
            hash,
            &new_change.signature,
            vault,
        )
        .await?
        {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        Ok(())
    }
}
