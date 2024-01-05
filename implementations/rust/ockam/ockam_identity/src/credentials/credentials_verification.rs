use tracing::{debug, warn};

use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::VaultForVerifyingSignatures;

use crate::identities::AttributesEntry;
use crate::models::{
    CredentialAndPurposeKey, CredentialData, Identifier, PurposePublicKey, VersionedData,
};
use crate::utils::now;
use crate::{
    CredentialAndPurposeKeyData, IdentityAttributesRepository, IdentityError,
    PurposeKeyVerification, TimestampInSeconds,
};

/// We allow Credentials to be created in the future related to this machine's time due to
/// possible time dyssynchronization
const MAX_ALLOWED_TIME_DRIFT: TimestampInSeconds = TimestampInSeconds(5);

/// Service for managing [`Credential`]s
pub struct CredentialsVerification {
    purpose_keys_verification: Arc<PurposeKeyVerification>,
    verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
    identities_attributes_repository: Arc<dyn IdentityAttributesRepository>,
}

impl CredentialsVerification {
    ///Constructor
    pub fn new(
        purpose_keys_verification: Arc<PurposeKeyVerification>,
        verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
        identities_attributes_repository: Arc<dyn IdentityAttributesRepository>,
    ) -> Self {
        Self {
            purpose_keys_verification,
            verifying_vault,
            identities_attributes_repository,
        }
    }
}

impl CredentialsVerification {
    /// Verify a [`Credential`]
    // TODO: Move to CredentialsVerification
    pub async fn verify_credential(
        &self,
        expected_subject: Option<&Identifier>,
        authorities: &[Identifier],
        credential_and_purpose_key: &CredentialAndPurposeKey,
    ) -> Result<CredentialAndPurposeKeyData> {
        debug!("verify purpose key attestation");
        let purpose_key_data = self
            .purpose_keys_verification
            .verify_purpose_key_attestation(
                None,
                &credential_and_purpose_key.purpose_key_attestation,
            )
            .await?;

        debug!("verify issuer");
        if !authorities.contains(&purpose_key_data.subject) {
            warn!(
                "unknown authority on a credential: {}. Accepted authorities: {:?}",
                purpose_key_data.subject, authorities
            );
            return Err(IdentityError::UnknownAuthority)?;
        }

        debug!("verify purpose key type");
        let public_key = match purpose_key_data.public_key.clone() {
            PurposePublicKey::SecureChannelStatic(_) => {
                return Err(IdentityError::InvalidKeyType)?;
            }

            PurposePublicKey::CredentialSigning(public_key) => public_key,
        };

        debug!("verify signature");
        let public_key = public_key.into();
        let versioned_data_hash = self
            .verifying_vault
            .sha256(&credential_and_purpose_key.credential.data)
            .await?;

        let signature = credential_and_purpose_key
            .credential
            .signature
            .clone()
            .into();

        if !self
            .verifying_vault
            .verify_signature(&public_key, &versioned_data_hash.0, &signature)
            .await?
        {
            return Err(IdentityError::CredentialVerificationFailed)?;
        }

        let versioned_data: VersionedData =
            minicbor::decode(&credential_and_purpose_key.credential.data)?;

        let credential_data = CredentialData::get_data(&versioned_data)?;

        debug!(
            "verify subject {:?}. Expected {:?}",
            credential_data.subject, expected_subject
        );
        if credential_data.subject.is_none() {
            // Currently unsupported
            return Err(IdentityError::CredentialVerificationFailed)?;
        }

        if credential_data.subject.is_none() && credential_data.subject_latest_change_hash.is_none()
        {
            // At least one should be always present, otherwise it's unclear who this credential belongs to
            return Err(IdentityError::CredentialVerificationFailed)?;
        }

        if expected_subject.is_some() && credential_data.subject.as_ref() != expected_subject {
            // We expected credential that belongs to someone else
            return Err(IdentityError::CredentialVerificationFailed)?;
        }

        debug!("verify dates");
        if credential_data.created_at < purpose_key_data.created_at {
            // Credential validity time range should be inside the purpose key validity time range
            return Err(IdentityError::CredentialVerificationFailed)?;
        }

        if credential_data.expires_at > purpose_key_data.expires_at {
            // Credential validity time range should be inside the purpose key validity time range
            return Err(IdentityError::CredentialVerificationFailed)?;
        }

        let now = now()?;

        if credential_data.created_at > now
            && credential_data.created_at - now > MAX_ALLOWED_TIME_DRIFT
        {
            // Credential can't be created in the future
            return Err(IdentityError::CredentialVerificationFailed)?;
        }

        if credential_data.expires_at < now {
            // Credential expired
            return Err(IdentityError::CredentialVerificationFailed)?;
        }

        if let Some(_subject_latest_change_hash) = &credential_data.subject_latest_change_hash {
            // TODO: Check how that aligns with the ChangeHistory of the subject that we have in the storage
            //     For example, if we just established a secure channel with that subject,
            //     latest_change_hash MUST be equal to the one in present ChangeHistory.
            //     If credential_data.subject_latest_change_hash equals to some older value from the
            //     subject's ChangeHistory, that means that subject hasn't updated its Credentials
            //     after the Identity Key rotation, which is suspicious, such Credential should be rejected
            //     If credential_data.subject_latest_change_hash equals to some future value that we haven't yet
            //     observed, than subject should had presented its newer Changes as well. We should
            //     reject such Credential, unless we have cases where subject may not had an opportunity
            //     to present its newer Changes (e.g., if we receive its Credential from someone else).
            //     In such cases some limited tolerance may be introduced.
        }

        // FIXME: Verify if given authority is allowed to issue credentials with given Schema <-- Should be handled somewhere in the TrustContext
        // FIXME: Verify if Schema aligns with Attributes <-- Should be handled somewhere in the TrustContext

        Ok(CredentialAndPurposeKeyData {
            credential_data,
            purpose_key_data,
        })
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

        let map = credential_data.credential_data.subject_attributes.map;
        let map: BTreeMap<_, _> = map
            .into_iter()
            .map(|(k, v)| (Vec::<u8>::from(k), Vec::<u8>::from(v)))
            .collect();

        self.identities_attributes_repository
            .put_attributes(
                subject,
                AttributesEntry::new(
                    map,
                    now()?,
                    Some(credential_data.credential_data.expires_at),
                    Some(credential_data.purpose_key_data.subject),
                ),
            )
            .await?;

        Ok(())
    }
}
