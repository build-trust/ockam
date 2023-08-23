use super::super::models::{Change, ChangeData, ChangeHash, ChangeSignature, CHANGE_HASH_LEN};
use super::super::verified_change::VerifiedChange;
use super::super::{Identity, IdentityError};
use arrayref::array_ref;

use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::constants::SHA256_LENGTH;
use ockam_vault::{PublicKey, SecretType, Signature, VerifyingVault};

struct ChangeDetails {
    version: u8,
    change_hash: ChangeHash,
    change_full_hash: [u8; SHA256_LENGTH],
    change_data: ChangeData,
}

impl Identity {
    pub(crate) fn compute_change_hash_from_hash(hash: [u8; 32]) -> Result<ChangeHash> {
        Ok(ChangeHash::new(*array_ref!(hash, 0, CHANGE_HASH_LEN)))
    }

    /// Check consistency of changes that are being added
    pub async fn check_entire_consistency(
        changes: &[Change],
        verifying_vault: Arc<dyn VerifyingVault>,
    ) -> Result<Vec<VerifiedChange>> {
        let to_be_verified_changes =
            Self::check_consistency(None, changes, verifying_vault).await?;

        Ok(to_be_verified_changes)
    }

    async fn get_change_details(
        change: &Change,
        vault: Arc<dyn VerifyingVault>,
    ) -> Result<ChangeDetails> {
        let change_full_hash = vault.sha256(&change.data).await?;
        let change_hash = Self::compute_change_hash_from_hash(change_full_hash)?;
        let versioned_data = change.get_versioned_data()?;

        if versioned_data.version != 1 {
            return Err(IdentityError::UnknownIdentityVersion.into());
        }

        let change_data = ChangeData::get_data(&versioned_data)?;

        Ok(ChangeDetails {
            version: versioned_data.version,
            change_hash,
            change_full_hash,
            change_data,
        })
    }

    /// Check consistency of changes that are being added
    async fn check_consistency(
        last_known_change: Option<&Change>,
        new_changes: &[Change],
        vault: Arc<dyn VerifyingVault>,
    ) -> Result<Vec<VerifiedChange>> {
        let mut to_be_verified_changes = Vec::with_capacity(new_changes.len());

        let mut previous_change_details = match last_known_change {
            Some(previous_change) => {
                Some(Self::get_change_details(previous_change, vault.clone()).await?)
            }
            None => None,
        };

        for change in new_changes.iter() {
            let change_details = Self::get_change_details(change, vault.clone()).await?;

            if let Some(previous_change_details) = previous_change_details {
                if previous_change_details.version > change_details.version {
                    // Version downgrade
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }

                if previous_change_details.change_data.created_at
                    > change_details.change_data.created_at
                {
                    // The older key can't be created after the newer
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }

                // This is intentionally allowed:
                // change_details.change_data.created_at > previous_change_details.change_data.expires_at

                if Some(&previous_change_details.change_hash)
                    != change_details.change_data.previous_change.as_ref()
                {
                    // Corrupted changes sequence
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }
            } else if change_details.change_data.previous_change.is_some() {
                // Should be empty
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
        vault: Arc<dyn VerifyingVault>,
    ) -> Result<()> {
        if to_be_verified_changes.len() != changes.len() {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        for i in 0..to_be_verified_changes.len() {
            let last_verified_change = if i == 0 {
                None
            } else {
                Some(&to_be_verified_changes[i - 1])
            };

            let new_change = &changes[i];
            Self::verify_change_signatures(last_verified_change, new_change, vault.clone()).await?
        }
        Ok(())
    }

    async fn verify_change_signature(
        public_key: &PublicKey,
        hash: [u8; 32],
        signature: &ChangeSignature,
        vault: Arc<dyn VerifyingVault>,
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
    async fn verify_change_signatures(
        last_verified_change: Option<&VerifiedChange>,
        new_change: &Change,
        vault: Arc<dyn VerifyingVault>,
    ) -> Result<()> {
        let new_change_details = Self::get_change_details(new_change, vault.clone()).await?;

        if let Some(last_verified_change) = last_verified_change {
            if let Some(previous_signature) = &new_change.previous_signature {
                if !Self::verify_change_signature(
                    last_verified_change.primary_public_key(),
                    new_change_details.change_full_hash,
                    previous_signature,
                    vault.clone(),
                )
                .await?
                {
                    return Err(IdentityError::IdentityVerificationFailed.into());
                }
            } else {
                // Previous signature should be present if it's not the first change
                return Err(IdentityError::IdentityVerificationFailed.into());
            }
        }

        if !Self::verify_change_signature(
            &new_change_details
                .change_data
                .primary_public_key
                .clone()
                .into(),
            new_change_details.change_full_hash,
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
