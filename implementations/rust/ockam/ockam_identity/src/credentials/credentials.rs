use crate::models::{CredentialData, PurposeKeyAttestationData};
use crate::{CredentialsCreation, CredentialsVerification, IdentitiesRepository, PurposeKeys};

use ockam_core::compat::sync::Arc;
use ockam_vault::{SigningVault, VerifyingVault};

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
    credential_vault: Arc<dyn SigningVault>,
    verifying_vault: Arc<dyn VerifyingVault>,
    purpose_keys: Arc<PurposeKeys>,
    identities_repository: Arc<dyn IdentitiesRepository>,
}

impl Credentials {
    ///Constructor
    pub fn new(
        credential_vault: Arc<dyn SigningVault>,
        verifying_vault: Arc<dyn VerifyingVault>,
        purpose_keys: Arc<PurposeKeys>,
        identities_repository: Arc<dyn IdentitiesRepository>,
    ) -> Self {
        Self {
            credential_vault,
            verifying_vault,
            purpose_keys,
            identities_repository,
        }
    }

    /// [`PurposeKeys`]
    pub fn purpose_keys(&self) -> Arc<PurposeKeys> {
        self.purpose_keys.clone()
    }

    /// [`IdentitiesRepository`]
    pub fn identities_repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.identities_repository.clone()
    }

    /// Return [`CredentialsCreation`]
    pub fn credentials_creation(&self) -> Arc<CredentialsCreation> {
        Arc::new(CredentialsCreation::new(
            self.purpose_keys.purpose_keys_creation(),
            self.credential_vault.clone(),
            self.verifying_vault.clone(),
            self.identities_repository.clone(),
        ))
    }

    /// Return [`CredentialsVerification`]
    pub fn credentials_verification(&self) -> Arc<CredentialsVerification> {
        Arc::new(CredentialsVerification::new(
            self.purpose_keys.purpose_keys_verification(),
            self.verifying_vault.clone(),
            self.identities_repository.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::identities::identities;
    use crate::models::SchemaId;
    use crate::Attributes;
    use ockam_core::compat::collections::BTreeMap;
    use ockam_core::Result;
    use std::time::Duration;

    #[tokio::test]
    async fn test_issue_credential() -> Result<()> {
        let identities = identities();
        let creation = identities.identities_creation();

        let issuer = creation.create_identity().await?;
        let subject = creation.create_identity().await?;
        let credentials = identities.credentials();

        let mut map: BTreeMap<Vec<u8>, Vec<u8>> = Default::default();
        map.insert(b"key".to_vec(), b"value".to_vec());
        let subject_attributes = Attributes {
            schema: SchemaId(1),
            map,
        };

        let credential = credentials
            .credentials_creation()
            .issue_credential(
                issuer.identifier(),
                subject.identifier(),
                subject_attributes,
                Duration::from_secs(60),
            )
            .await?;

        let _res = credentials
            .credentials_verification()
            .verify_credential(
                Some(subject.identifier()),
                &[issuer.identifier().clone()],
                &credential,
            )
            .await?;

        Ok(())
    }
}
