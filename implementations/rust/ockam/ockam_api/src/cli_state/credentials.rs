use ockam::identity::models::{ChangeHistory, CredentialAndPurposeKey};
use ockam::identity::{AttributesEntry, Identifier, Identity};

use crate::cli_state::{CliState, CliStateError};

use super::Result;

impl CliState {
    /// Store a credential inside the local database
    /// This function stores both the credential as a named entity
    /// and the identity attributes in another table.
    /// TODO: normalize the storage so that the data is only represented once
    pub async fn store_credential(
        &self,
        name: &str,
        issuer: &Identity,
        credential: CredentialAndPurposeKey,
    ) -> Result<()> {
        // store the subject attributes
        let credential_data = credential.get_credential_data()?;
        let identity_attributes_repository = self.identity_attributes_repository().await?;
        if let Some(subject) = credential_data.subject {
            let attributes_entry = AttributesEntry::new(
                credential_data
                    .subject_attributes
                    .map
                    .into_iter()
                    .map(|(k, v)| (k.to_vec(), v.to_vec()))
                    .collect(),
                credential_data.created_at,
                Some(credential_data.expires_at),
                Some(issuer.identifier().clone()),
            );
            identity_attributes_repository
                .put_attributes(&subject, attributes_entry)
                .await?;
        }

        // store the credential itself
        let credentials_repository = self.credentials_repository().await?;
        credentials_repository
            .store_credential(name, issuer, credential)
            .await?;
        Ok(())
    }

    /// Return a credential given its name
    pub async fn get_credential_by_name(&self, name: &str) -> Result<NamedCredential> {
        match self
            .credentials_repository()
            .await?
            .get_credential(name)
            .await?
        {
            Some(credential) => Ok(credential),
            None => Err(CliStateError::ResourceNotFound {
                name: name.to_string(),
                resource: "credential".into(),
            }),
        }
    }

    pub async fn get_credentials(&self) -> Result<Vec<NamedCredential>> {
        Ok(self
            .credentials_repository()
            .await?
            .get_credentials()
            .await?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedCredential {
    name: String,
    issuer_identifier: Identifier,
    issuer_change_history: ChangeHistory,
    credential: CredentialAndPurposeKey,
}

impl NamedCredential {
    pub fn new(name: &str, issuer: &Identity, credential: CredentialAndPurposeKey) -> Self {
        Self::make(
            name,
            issuer.identifier().clone(),
            issuer.change_history().clone(),
            credential,
        )
    }

    pub fn make(
        name: &str,
        issuer_identifier: Identifier,
        issuer_change_history: ChangeHistory,
        credential: CredentialAndPurposeKey,
    ) -> Self {
        Self {
            name: name.to_string(),
            issuer_identifier,
            issuer_change_history,
            credential,
        }
    }
}

impl NamedCredential {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn issuer_identifier(&self) -> Identifier {
        self.issuer_identifier.clone()
    }

    pub async fn issuer_identity(&self) -> Result<Identity> {
        Ok(Identity::create_from_change_history(&self.issuer_change_history).await?)
    }

    pub fn issuer_change_history(&self) -> ChangeHistory {
        self.issuer_change_history.clone()
    }

    pub fn credential_and_purpose_key(&self) -> CredentialAndPurposeKey {
        self.credential.clone()
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::time::Duration;

    use ockam::identity::models::CredentialSchemaIdentifier;
    use ockam::identity::utils::AttributesBuilder;
    use ockam::identity::{identities, Identities};

    use super::*;

    #[tokio::test]
    async fn test_cli_spaces() -> Result<()> {
        let cli = CliState::test().await?;
        let identities = identities().await?;
        let issuer_identifier = identities.identities_creation().create_identity().await?;
        let issuer = identities.get_identity(&issuer_identifier).await?;
        let credential = create_credential(identities, &issuer_identifier).await?;

        // a credential can be stored and retrieved by name
        cli.store_credential("name1", &issuer, credential.clone())
            .await?;
        let result = cli.get_credential_by_name("name1").await?;
        assert_eq!(result.name(), "name1".to_string());
        assert_eq!(result.issuer_identifier(), issuer_identifier);
        assert_eq!(result.issuer_change_history(), *issuer.change_history());
        assert_eq!(result.credential_and_purpose_key(), credential);

        Ok(())
    }

    /// HELPERS
    async fn create_credential(
        identities: Arc<Identities>,
        issuer: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        let subject = identities.identities_creation().create_identity().await?;

        let attributes = AttributesBuilder::with_schema(CredentialSchemaIdentifier(1))
            .with_attribute("name".as_bytes().to_vec(), b"value".to_vec())
            .build();

        Ok(identities
            .credentials()
            .credentials_creation()
            .issue_credential(issuer, &subject, attributes, Duration::from_secs(1))
            .await?)
    }
}
