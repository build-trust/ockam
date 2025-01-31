use core::time::Duration;
use tracing::trace;

use crate::authenticator::credential_issuer::CredentialIssuer;
use crate::authenticator::direct::AccountAuthorityInfo;
use crate::authenticator::AuthorityMembersRepository;
use ockam::identity::{Credentials, Identifier, IdentitiesAttributes};
use ockam_core::api::{Method, Request, Response};
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
    type Message = Request<Vec<u8>>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        let secure_channel_info = match SecureChannelLocalInfo::find_info(m.local_message()) {
            Ok(secure_channel_info) => secure_channel_info,
            Err(_e) => {
                let resp =
                    Response::bad_request_no_request("secure channel required").encode_body()?;
                c.send(m.return_route().clone(), resp).await?;
                return Ok(());
            }
        };

        let from = Identifier::from(secure_channel_info.their_identifier());
        let return_route = m.return_route().clone();
        let request = m.into_body()?;
        let header = request.header();
        trace! {
            target: "credential_issuer",
            from   = %from,
            id     = %header.id(),
            method = ?header.method(),
            path   = %header.path(),
            body   = %header.has_body(),
            "request"
        }
        let res = match (header.method(), header.path()) {
            (Some(Method::Post), "/") | (Some(Method::Post), "/credential") => {
                match self.credential_issuer.issue_credential(&from).await {
                    Ok(Some(crd)) => Response::ok()
                        .with_headers(header)
                        .body(crd)
                        .encode_body()?,
                    Ok(None) => Response::forbidden(header, "unauthorized member").encode_body()?,
                    Err(error) => {
                        Response::internal_error(header, &error.to_string()).encode_body()?
                    }
                }
            }
            _ => Response::unknown_path(header).encode_body()?,
        };

        c.send(return_route, res).await
    }
}
