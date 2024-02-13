use core::time::Duration;

use crate::authenticator::AuthorityMembersRepository;
use ockam::identity::models::{CredentialAndPurposeKey, CredentialSchemaIdentifier};
use ockam::identity::utils::AttributesBuilder;
use ockam::identity::{Attributes, Credentials, Identifier};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

/// Legacy value, should be removed when all clients are updated to the latest version
pub const TRUST_CONTEXT_ID: &[u8] = b"trust_context_id";

/// Identifier for the schema of a project credential
pub const PROJECT_MEMBER_SCHEMA: CredentialSchemaIdentifier = CredentialSchemaIdentifier(1);

/// Maximum duration for a valid credential in seconds (30 days)
pub const DEFAULT_CREDENTIAL_VALIDITY: Duration = Duration::from_secs(30 * 24 * 3600);

/// This struct runs as a Worker to issue credentials based on a request/response protocol
pub struct CredentialIssuer {
    members: Arc<dyn AuthorityMembersRepository>,
    credentials: Arc<Credentials>,
    issuer: Identifier,
    subject_attributes: Attributes,
    credential_ttl: Duration,
}

impl CredentialIssuer {
    /// Create a new credentials issuer
    #[instrument(skip_all, fields(issuer = %issuer, project_identifier = project_identifier.clone(), credential_ttl = credential_ttl.map_or("n/a".to_string(), |d| d.as_secs().to_string())))]
    pub fn new(
        members: Arc<dyn AuthorityMembersRepository>,
        credentials: Arc<Credentials>,
        issuer: &Identifier,
        project_identifier: Option<String>, // Legacy value, should be removed when all clients are updated to the latest version
        credential_ttl: Option<Duration>,
    ) -> Self {
        let subject_attributes = AttributesBuilder::with_schema(PROJECT_MEMBER_SCHEMA);
        let subject_attributes = if let Some(project_identifier) = project_identifier {
            // Legacy value, should be removed when all clients are updated to the latest version
            subject_attributes.with_attribute(
                TRUST_CONTEXT_ID.to_vec(),
                project_identifier.as_bytes().to_vec(),
            )
        } else {
            subject_attributes
        };
        let subject_attributes = subject_attributes.build();

        Self {
            members,
            credentials,
            issuer: issuer.clone(),
            subject_attributes,
            credential_ttl: credential_ttl.unwrap_or(DEFAULT_CREDENTIAL_VALIDITY),
        }
    }

    #[instrument(skip_all, fields(subject = %subject))]
    pub async fn issue_credential(
        &self,
        subject: &Identifier,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        let member = match self.members.get_member(subject).await? {
            Some(member) => member,
            None => return Ok(None),
        };

        let mut subject_attributes = self.subject_attributes.clone();
        for (key, value) in member.attributes().iter() {
            subject_attributes
                .map
                .insert(key.clone().into(), value.clone().into());
        }

        let credential = self
            .credentials
            .credentials_creation()
            .issue_credential(
                &self.issuer,
                subject,
                subject_attributes,
                self.credential_ttl,
            )
            .await?;

        info!("Successfully issued a credential for {}", subject);

        Ok(Some(credential))
    }
}
