use minicbor::Decoder;
use tracing::trace;

use ockam_core::api::{Method, Request, Response};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{api, Result, Route, Routed, Worker};
use ockam_node::{Context, RpcClient};

use crate::alloc::string::ToString;
use crate::credential::Credential;
use crate::identity::IdentityIdentifier;
use crate::{CredentialData, Identities, IdentitySecureChannelLocalInfo, PROJECT_MEMBER_SCHEMA};

/// Legacy id for a trust context, it used to be 'project_id', not it is the more general 'trust_context_id'
/// TODO: DEPRECATE - Removing PROJECT_ID attribute in favor of TRUST_CONTEXT_ID
pub const LEGACY_ID: &str = "project_id";
/// Name of the attribute identifying the trust context for that attribute, meaning
/// from which set of trusted authorities the attribute comes from
pub const TRUST_CONTEXT_ID: &str = "trust_context_id";

/// This struct runs as a Worker to issue credentials based on a request/response protocol
pub struct CredentialsIssuer {
    identities: Arc<Identities>,
    issuer: IdentityIdentifier,
    trust_context: String,
}

impl CredentialsIssuer {
    /// Create a new credentials issuer
    pub async fn new(
        identities: Arc<Identities>,
        issuer: IdentityIdentifier,
        trust_context: String,
    ) -> Result<Self> {
        Ok(Self {
            identities,
            issuer,
            trust_context,
        })
    }

    async fn issue_credential(&self, from: &IdentityIdentifier) -> Result<Option<Credential>> {
        match self
            .identities
            .repository()
            .as_attributes_reader()
            .get_attributes(from)
            .await?
        {
            Some(entry) => {
                let crd = entry
                    .attrs()
                    .iter()
                    .fold(
                        CredentialData::builder(from.clone(), self.issuer.clone())
                            .with_schema(PROJECT_MEMBER_SCHEMA),
                        |crd, (a, v)| crd.with_attribute(a, v),
                    )
                    .with_attribute(LEGACY_ID, self.trust_context.as_bytes()) // TODO: DEPRECATE - Removing PROJECT_ID attribute in favor of TRUST_CONTEXT_ID
                    .with_attribute(TRUST_CONTEXT_ID, self.trust_context.as_bytes());
                Ok(Some(
                    self.identities
                        .credentials()
                        .issue_credential(&self.issuer, crd.build()?)
                        .await?,
                ))
            }
            None => Ok(None),
        }
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
    pub async fn credential(&self) -> Result<Credential> {
        self.client.request(&Request::post("/")).await
    }
}
