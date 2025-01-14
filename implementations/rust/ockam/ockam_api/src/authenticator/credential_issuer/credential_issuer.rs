use core::time::Duration;

use crate::authenticator::direct::AccountAuthorityInfo;
use crate::authenticator::AuthorityMembersRepository;
use ockam::identity::models::{CredentialAndPurposeKey, CredentialSchemaIdentifier};
use ockam::identity::utils::AttributesBuilder;
use ockam::identity::{Attributes, Credentials, Identifier, IdentitiesAttributes};
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
    identities_attributes: Arc<IdentitiesAttributes>,
    credentials: Arc<Credentials>,
    issuer: Identifier,
    subject_attributes: Attributes,
    credential_ttl: Duration,

    account_authority: Option<AccountAuthorityInfo>,
}

impl CredentialIssuer {
    /// Create a new credentials issuer
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all, fields(issuer = %issuer, project_identifier = project_identifier.clone(), credential_ttl = credential_ttl.map_or("n/a".to_string(), |d| d.as_secs().to_string())))]
    pub fn new(
        members: Arc<dyn AuthorityMembersRepository>,
        identities_attributes: Arc<IdentitiesAttributes>,
        credentials: Arc<Credentials>,
        issuer: &Identifier,
        project_identifier: String,
        credential_ttl: Option<Duration>,
        account_authority: Option<AccountAuthorityInfo>,
        disable_trust_context_id: bool,
    ) -> Self {
        let subject_attributes = AttributesBuilder::with_schema(PROJECT_MEMBER_SCHEMA);
        let subject_attributes = if !disable_trust_context_id {
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
            identities_attributes,
            credentials,
            issuer: issuer.clone(),
            subject_attributes,
            credential_ttl: credential_ttl.unwrap_or(DEFAULT_CREDENTIAL_VALIDITY),
            account_authority,
        }
    }

    #[instrument(skip_all, fields(subject = %subject))]
    pub async fn issue_credential(
        &self,
        subject: &Identifier,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        // Check if it has a valid project admin credential
        if let Some(info) = self.account_authority.as_ref() {
            if let Some(attrs) = self
                .identities_attributes
                .get_attributes(subject, info.account_authority())
                .await?
            {
                if attrs.attrs().get("project".as_bytes())
                    == Some(&info.project_identifier().as_bytes().to_vec())
                {
                    let mut subject_attributes = self.subject_attributes.clone();
                    subject_attributes
                        .map
                        .insert(b"ockam-relay".to_vec().into(), b"*".to_vec().into());
                    subject_attributes.map.insert(
                        b"ockam-tls-certificate".to_vec().into(),
                        b"true".to_vec().into(),
                    );
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
                    info!("Successfully issued a credential for admin {}", subject);

                    return Ok(Some(credential));
                }
            }
        }

        // Otherwise, check if it's a member managed by this authority

        let member = match self.members.get_member(&self.issuer, subject).await? {
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
