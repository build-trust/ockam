use crate::models::{Attributes, CredentialAndPurposeKey, CredentialSchemaIdentifier, Identifier};
use crate::utils::AttributesBuilder;
use crate::{Credentials, IdentitiesRepository, IdentitySecureChannelLocalInfo};

use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::String;
use ockam_core::compat::string::ToString;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{Result, Routed, Worker};
use ockam_node::Context;

use core::time::Duration;
use minicbor::Decoder;
use tracing::trace;

/// Name of the attribute identifying the trust context for that attribute, meaning
/// from which set of trusted authorities the attribute comes from
pub const TRUST_CONTEXT_ID: &[u8] = b"trust_context_id";

/// The same as above but in string format
pub const TRUST_CONTEXT_ID_UTF8: &str = "trust_context_id";

/// Identifier for the schema of a project credential
pub const PROJECT_MEMBER_SCHEMA: CredentialSchemaIdentifier = CredentialSchemaIdentifier(1);

/// Maximum duration for a valid credential in seconds (30 days)
pub const MAX_CREDENTIAL_VALIDITY: Duration = Duration::from_secs(30 * 24 * 3600);

/// This struct runs as a Worker to issue credentials based on a request/response protocol
pub struct CredentialsIssuer {
    identities_repository: Arc<dyn IdentitiesRepository>,
    credentials: Arc<Credentials>,
    issuer: Identifier,
    subject_attributes: Attributes,
}

impl CredentialsIssuer {
    /// Create a new credentials issuer
    pub fn new(
        identities_repository: Arc<dyn IdentitiesRepository>,
        credentials: Arc<Credentials>,
        issuer: &Identifier,
        trust_context: String,
    ) -> Self {
        let subject_attributes = AttributesBuilder::with_schema(PROJECT_MEMBER_SCHEMA)
            .with_attribute(TRUST_CONTEXT_ID.to_vec(), trust_context.as_bytes().to_vec())
            .build();

        Self {
            identities_repository,
            credentials,
            issuer: issuer.clone(),
            subject_attributes,
        }
    }

    async fn issue_credential(
        &self,
        subject: &Identifier,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        let entry = match self
            .identities_repository
            .as_attributes_reader()
            .get_attributes(subject)
            .await?
        {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let mut subject_attributes = self.subject_attributes.clone();
        for (key, value) in entry.attrs().iter() {
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
                MAX_CREDENTIAL_VALIDITY,
            )
            .await?;

        Ok(Some(credential))
    }
}

#[ockam_core::worker]
impl Worker for CredentialsIssuer {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let from = i.their_identity_id();
            let mut dec = Decoder::new(m.as_body());
            let req: RequestHeader = dec.decode()?;
            trace! {
                target: "ockam_identity::credentials::credential_issuer",
                from   = %from,
                id     = %req.id(),
                method = ?req.method(),
                path   = %req.path(),
                body   = %req.has_body(),
                "request"
            }
            let res = match (req.method(), req.path()) {
                (Some(Method::Post), "/") | (Some(Method::Post), "/credential") => {
                    match self.issue_credential(&from).await {
                        Ok(Some(crd)) => Response::ok(&req).body(crd).to_vec()?,
                        Ok(None) => {
                            // Again, this has already been checked by the access control, so if we
                            // reach this point there is an error actually.
                            Response::forbidden(&req, "unauthorized member").to_vec()?
                        }
                        Err(error) => {
                            Response::internal_error(&req, &error.to_string()).to_vec()?
                        }
                    }
                }
                _ => Response::unknown_path(&req).to_vec()?,
            };
            c.send(m.return_route(), res).await
        } else {
            secure_channel_required(c, m).await
        }
    }
}

/// Return a response on the return route stating that a secure channel is needed to access
/// the service
pub async fn secure_channel_required(c: &mut Context, m: Routed<Vec<u8>>) -> Result<()> {
    // This was, actually, already checked by the access control. So if we reach this point
    // it means there is a bug.  Also, if it' already checked, we should receive the Peer'
    // identity, not an Option to the peer' identity.
    let mut dec = Decoder::new(m.as_body());
    let req: RequestHeader = dec.decode()?;
    let res = Response::forbidden(&req, "secure channel required").to_vec()?;
    c.send(m.return_route(), res).await
}
