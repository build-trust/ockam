use core::time::Duration;
use minicbor::Decoder;
use tracing::trace;

use crate::authenticator::credential_issuer::CredentialIssuer;
use crate::authenticator::direct::AccountAuthorityInfo;
use crate::authenticator::AuthorityMembersRepository;
use ockam::identity::{Credentials, Identifier, IdentitiesAttributes};
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{Result, Routed, SecureChannelLocalInfo, Worker};
use ockam_node::Context;

/// This struct runs as a Worker to issue credentials based on a request/response protocol
pub struct CredentialIssuerWorker {
    credential_issuer: CredentialIssuer,
}

impl CredentialIssuerWorker {
    /// Create a new credentials issuer
    #[allow(clippy::too_many_arguments)]
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
        Self {
            credential_issuer: CredentialIssuer::new(
                members,
                identities_attributes,
                credentials,
                issuer,
                project_identifier,
                credential_ttl,
                account_authority,
                disable_trust_context_id,
            ),
        }
    }
}

#[ockam_core::worker]
impl Worker for CredentialIssuerWorker {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        let secure_channel_info = match SecureChannelLocalInfo::find_info(m.local_message()) {
            Ok(secure_channel_info) => secure_channel_info,
            Err(_e) => {
                let resp = Response::bad_request_no_request("secure channel required").to_vec()?;
                c.send(m.return_route(), resp).await?;
                return Ok(());
            }
        };

        let from = Identifier::from(secure_channel_info.their_identifier());
        let return_route = m.return_route();
        let body = m.into_body()?;
        let mut dec = Decoder::new(&body);
        let req: RequestHeader = dec.decode()?;
        trace! {
            target: "credential_issuer",
            from   = %from,
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }
        let res = match (req.method(), req.path()) {
            (Some(Method::Post), "/") | (Some(Method::Post), "/credential") => {
                match self.credential_issuer.issue_credential(&from).await {
                    Ok(Some(crd)) => Response::ok().with_headers(&req).body(crd).to_vec()?,
                    Ok(None) => Response::forbidden(&req, "unauthorized member").to_vec()?,
                    Err(error) => Response::internal_error(&req, &error.to_string()).to_vec()?,
                }
            }
            _ => Response::unknown_path(&req).to_vec()?,
        };

        c.send(return_route, res).await
    }
}
