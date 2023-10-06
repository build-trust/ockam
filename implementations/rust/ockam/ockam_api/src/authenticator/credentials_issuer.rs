use ockam::identity::models::{CredentialAndPurposeKey, CredentialSchemaIdentifier, Identifier};
use ockam::identity::utils::AttributesBuilder;
use ockam::identity::{Credentials, IdentitySecureChannelLocalInfo};
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::ToString;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{Result, Routed, Worker};
use ockam_node::Context;

use crate::authenticator::{secure_channel_required, MembersStorage};
use core::time::Duration;
use minicbor::Decoder;
use tracing::trace;

/// Identifier for the schema of a project credential
pub const PROJECT_MEMBER_SCHEMA: CredentialSchemaIdentifier = CredentialSchemaIdentifier(1);

/// Maximum duration for a valid credential in seconds (30 days)
pub const MAX_CREDENTIAL_VALIDITY: Duration = Duration::from_secs(30 * 24 * 3600);

/// This struct runs as a Worker to issue credentials based on a request/response protocol
pub struct CredentialsIssuer {
    members_storage: Arc<dyn MembersStorage>,
    credentials: Arc<Credentials>,
    issuer: Identifier,
}

impl CredentialsIssuer {
    /// Create a new credentials issuer
    pub fn new(
        members_storage: Arc<dyn MembersStorage>,
        credentials: Arc<Credentials>,
        issuer: &Identifier,
    ) -> Self {
        Self {
            members_storage,
            credentials,
            issuer: issuer.clone(),
        }
    }

    async fn issue_credential(
        &self,
        subject: &Identifier,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        let member = match self.members_storage.get_member(subject).await? {
            Some(member) => member,
            None => return Ok(None),
        };

        let mut builder = AttributesBuilder::with_schema(PROJECT_MEMBER_SCHEMA);
        for (key, value) in member.attributes() {
            builder = builder.with_attribute(key.clone(), value.clone());
        }

        let credential = self
            .credentials
            .credentials_creation()
            .issue_credential(
                &self.issuer,
                subject,
                builder.build(),
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
