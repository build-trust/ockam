use ockam_core::compat::sync::Arc;
use ockam_vault::{VaultForSigning, VaultForVerifyingSignatures};

use crate::models::{CredentialData, PurposeKeyAttestationData};
use crate::{
    CredentialsCreation, CredentialsVerification, IdentitiesCreation, IdentityAttributesRepository,
    PurposeKeys,
};

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
    credential_vault: Arc<dyn VaultForSigning>,
    verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
    purpose_keys: Arc<PurposeKeys>,
    identities_creation: Arc<IdentitiesCreation>,
    identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
}

impl Credentials {
    ///Constructor
    pub fn new(
        credential_vault: Arc<dyn VaultForSigning>,
        verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
        purpose_keys: Arc<PurposeKeys>,
        identities_creation: Arc<IdentitiesCreation>,
        identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
    ) -> Self {
        Self {
            credential_vault,
            verifying_vault,
            purpose_keys,
            identities_creation,
            identity_attributes_repository,
        }
    }

    /// [`PurposeKeys`]
    pub fn purpose_keys(&self) -> Arc<PurposeKeys> {
        self.purpose_keys.clone()
    }

    /// Return [`CredentialsCreation`]
    pub fn credentials_creation(&self) -> Arc<CredentialsCreation> {
        Arc::new(CredentialsCreation::new(
            self.purpose_keys.purpose_keys_creation(),
            self.credential_vault.clone(),
            self.verifying_vault.clone(),
            self.identities_creation.clone(),
        ))
    }

    /// Return [`CredentialsVerification`]
    pub fn credentials_verification(&self) -> Arc<CredentialsVerification> {
        Arc::new(CredentialsVerification::new(
            self.purpose_keys.purpose_keys_verification(),
            self.verifying_vault.clone(),
            self.identity_attributes_repository.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use minicbor::bytes::ByteVec;

    use ockam_core::compat::collections::BTreeMap;
    use ockam_core::Result;

    use crate::identities::identities;
    use crate::models::CredentialSchemaIdentifier;
    use crate::Attributes;

    #[tokio::test]
    async fn test_issue_credential() -> Result<()> {
        let identities = identities().await?;
        let creation = identities.identities_creation();

        let issuer = creation.create_identity().await?;
        let subject = creation.create_identity().await?;
        let credentials = identities.credentials();

        let mut map: BTreeMap<ByteVec, ByteVec> = Default::default();
        map.insert(b"key".to_vec().into(), b"value".to_vec().into());
        let subject_attributes = Attributes {
            schema: CredentialSchemaIdentifier(1),
            map,
        };

        let credential = credentials
            .credentials_creation()
            .issue_credential(
                &issuer,
                &subject,
                subject_attributes,
                Duration::from_secs(60 * 60),
            )
            .await?;

        println!("{}", hex::encode(minicbor::to_vec(&credential)?));

        let _res = credentials
            .credentials_verification()
            .verify_credential(Some(&subject), &[issuer.clone()], &credential)
            .await?;

        Ok(())
    }
}
