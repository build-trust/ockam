use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::VaultForVerifyingSignatures;

use crate::models::{Identifier, PurposeKeyAttestation, PurposeKeyAttestationData};
use crate::utils::now;
use crate::{IdentitiesReader, Identity, IdentityError, TimestampInSeconds};

/// We allow purpose keys to be created in the future related to this machine's time due to
/// possible time dyssynchronization
const MAX_ALLOWED_TIME_DRIFT: TimestampInSeconds = TimestampInSeconds(5);

/// This struct supports all the services related to identities
#[derive(Clone)]
pub struct PurposeKeyVerification {
    verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
    identities_reader: Arc<dyn IdentitiesReader>,
}

impl PurposeKeyVerification {
    /// Create a new identities module
    pub(crate) fn new(
        verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
        identities_reader: Arc<dyn IdentitiesReader>,
    ) -> Self {
        Self {
            verifying_vault,
            identities_reader,
        }
    }
}

impl PurposeKeyVerification {
    /// Verify a [`PurposeKeyAttestation`]
    pub async fn verify_purpose_key_attestation(
        &self,
        expected_subject: Option<&Identifier>,
        attestation: &PurposeKeyAttestation,
    ) -> Result<PurposeKeyAttestationData> {
        let versioned_data_hash = self.verifying_vault.sha256(&attestation.data).await?;

        let versioned_data = attestation.get_versioned_data()?;

        if versioned_data.version != 1 {
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        let purpose_key_data = PurposeKeyAttestationData::get_data(&versioned_data)?;

        if let Some(expected_subject) = expected_subject {
            if expected_subject != &purpose_key_data.subject {
                // We expected purpose key that belongs to someone else
                return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
            }
        }

        let change_history = self
            .identities_reader
            .get_identity(&purpose_key_data.subject)
            .await?;
        let identity = Identity::import_from_change_history(
            Some(&purpose_key_data.subject),
            change_history,
            self.verifying_vault.clone(),
        )
        .await?;

        let latest_change = identity.get_latest_change()?;

        // TODO: We should inspect purpose_key_data.subject_latest_change_hash, the possibilities are:
        //     1) It's equal to the latest Change we know about, this is the default case and
        //        this is the only case that the code below handles currently
        //     2) We haven't yet discovered that new Change, therefore we can't verify such PurposeKey
        //     3) It references previous Change from the known to us history, we might accept such
        //        PurposeKey, but not if the next Change has revoke_all_purpose_keys == true
        //     4) It references Change even older. IMO we shouldn't accept such PurposeKeys

        if &purpose_key_data.subject_latest_change_hash != latest_change.change_hash() {
            // Only verifying with the latest key is currently implemented, see the `TODO` above
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        if purpose_key_data.expires_at > latest_change.data().expires_at {
            // PurposeKey validity time range should be inside the identity key validity time range
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        if purpose_key_data.created_at < latest_change.data().created_at {
            // PurposeKey validity time range should be inside the identity key validity time range
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        let now = now()?;

        if purpose_key_data.created_at > now
            && purpose_key_data.created_at - now > MAX_ALLOWED_TIME_DRIFT
        {
            // PurposeKey can't be created in the future
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        if purpose_key_data.expires_at < now {
            // PurposeKey expired
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        let identity_public_key = latest_change.primary_public_key();

        if !self
            .verifying_vault
            .verify_signature(
                identity_public_key,
                &versioned_data_hash.0,
                &attestation.signature.clone().into(),
            )
            .await?
        {
            return Err(IdentityError::PurposeKeyAttestationVerificationFailed.into());
        }

        Ok(purpose_key_data)
    }
}
