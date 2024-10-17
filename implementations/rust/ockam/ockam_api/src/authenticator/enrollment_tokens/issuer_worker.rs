use either::Either;
use minicbor::Decoder;
use tracing::trace;

use ockam::identity::{Identifier, IdentitiesAttributes};
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::time::Duration;
use ockam_core::{Result, Routed, SecureChannelLocalInfo, Worker};
use ockam_node::Context;

use crate::authenticator::direct::types::CreateToken;
use crate::authenticator::direct::AccountAuthorityInfo;
use crate::authenticator::enrollment_tokens::EnrollmentTokenIssuer;
use crate::authenticator::{AuthorityEnrollmentTokenRepository, AuthorityMembersRepository};

pub struct EnrollmentTokenIssuerWorker {
    pub(super) issuer: EnrollmentTokenIssuer,
}

impl EnrollmentTokenIssuerWorker {
    pub fn new(
        tokens: Arc<dyn AuthorityEnrollmentTokenRepository>,
        members: Arc<dyn AuthorityMembersRepository>,
        identities_attributes: Arc<IdentitiesAttributes>,
        account_authority: Option<AccountAuthorityInfo>,
    ) -> Self {
        Self {
            issuer: EnrollmentTokenIssuer::new(
                tokens,
                members,
                identities_attributes,
                account_authority,
            ),
        }
    }
}

#[ockam_core::worker]
impl Worker for EnrollmentTokenIssuerWorker {
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
            target: "enrollment_token_issuer",
            from   = %from,
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }
        let res = match (req.method(), req.path()) {
            (Some(Method::Post), "/") | (Some(Method::Post), "/tokens") => {
                let att: CreateToken = dec.decode()?;
                let duration = att.ttl_secs().map(Duration::from_secs);
                let ttl_count = att.ttl_count();

                let res = self
                    .issuer
                    .issue_token(&from, att.into_owned_attributes(), duration, ttl_count)
                    .await?;

                match res {
                    Either::Left(otc) => Response::ok().with_headers(&req).body(&otc).to_vec()?,
                    Either::Right(error) => Response::forbidden(&req, &error.0).to_vec()?,
                }
            }
            _ => Response::unknown_path(&req).to_vec()?,
        };
        c.send(return_route, res).await
    }
}
