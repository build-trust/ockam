use minicbor::Decoder;
use std::collections::BTreeMap;
use std::time::Duration;
use tracing::trace;

use ockam_core::api::{Method, Request, Response};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{api, Result, Route, Routed, Worker};
use ockam_node::{Context, RpcClient};

use super::super::models::{Attributes, Credential, Identifier, SchemaId};
use super::super::{Credentials, IdentitiesRepository, Purpose, PurposeKey, PurposeKeys};
use crate::v2::models::CredentialAndPurposeKey;
use crate::IdentitySecureChannelLocalInfo;

/// Name of the attribute identifying the trust context for that attribute, meaning
/// from which set of trusted authorities the attribute comes from
pub const TRUST_CONTEXT_ID: &[u8] = b"trust_context_id";

/// Identifier for the schema of a project credential
pub const PROJECT_MEMBER_SCHEMA: SchemaId = SchemaId(1);

/// This struct runs as a Worker to issue credentials based on a request/response protocol
pub struct CredentialsIssuer {
    identities_repository: Arc<dyn IdentitiesRepository>,
    credentials: Arc<Credentials>,
    issuer: Identifier,
    subject_attributes: Attributes,
    purpose_key: PurposeKey,
}

impl CredentialsIssuer {
    /// Create a new credentials issuer
    pub async fn new(
        identities_repository: Arc<dyn IdentitiesRepository>,
        credentials: Arc<Credentials>,
        issuer: &Identifier,
        trust_context: String,
        purpose_keys: &PurposeKeys,
    ) -> Result<Self> {
        let mut subject_attributes: BTreeMap<Vec<u8>, Vec<u8>> = Default::default();
        subject_attributes.insert(TRUST_CONTEXT_ID.to_vec(), trust_context.as_bytes().to_vec());
        let subject_attributes = Attributes {
            schema: PROJECT_MEMBER_SCHEMA,
            map: subject_attributes,
        };

        // FIXME: Reuse the key
        let purpose_key = purpose_keys
            .create_purpose_key(issuer, Purpose::Credentials)
            .await?;

        Ok(Self {
            identities_repository,
            credentials,
            issuer: issuer.clone(),
            subject_attributes,
            purpose_key,
        })
    }

    async fn issue_credential(&self, subject: &Identifier) -> Result<Option<Credential>> {
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
            subject_attributes.map.insert(key.clone(), value.clone());
        }

        let credential = self
            .credentials
            .issue_credential(
                subject,
                &self.purpose_key,
                subject_attributes,
                Duration::from_secs(120), /* FIXME */
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
            let from: Identifier = i.their_identity_id().into();
            let mut dec = Decoder::new(m.as_body());
            let req: Request = dec.decode()?;
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
                        Ok(Some(crd)) => Response::ok(req.id()).body(crd).to_vec()?,
                        Ok(None) => {
                            // Again, this has already been checked by the access control, so if we
                            // reach this point there is an error actually.
                            api::forbidden(&req, "unauthorized member").to_vec()?
                        }
                        Err(error) => api::internal_error(&req, &error.to_string()).to_vec()?,
                    }
                }
                _ => api::unknown_path(&req).to_vec()?,
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
    let req: Request = dec.decode()?;
    let res = api::forbidden(&req, "secure channel required").to_vec()?;
    c.send(m.return_route(), res).await
}

/// Client for a credentials issuer
pub struct CredentialsIssuerClient {
    client: RpcClient,
}

impl CredentialsIssuerClient {
    /// Create a new credentials issuer client
    /// The route needs to be a secure channel
    pub async fn new(route: Route, ctx: &Context) -> Result<Self> {
        Ok(CredentialsIssuerClient {
            client: RpcClient::new(route, ctx).await?,
        })
    }

    /// Return a credential for the identity which initiated the secure channel
    pub async fn credential(&self) -> Result<CredentialAndPurposeKey> {
        self.client.request(&Request::post("/")).await
    }
}
