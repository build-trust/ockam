use either::Either;
use tracing::trace;

use ockam::identity::{Identifier, IdentitiesAttributes};
use ockam_core::api::{Method, Request, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::time::Duration;
use ockam_core::{Decodable, Result, Routed, SecureChannelLocalInfo, Worker};
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
        authority: &Identifier,
        tokens: Arc<dyn AuthorityEnrollmentTokenRepository>,
        members: Arc<dyn AuthorityMembersRepository>,
        identities_attributes: Arc<IdentitiesAttributes>,
        account_authority: Option<AccountAuthorityInfo>,
    ) -> Self {
        Self {
            issuer: EnrollmentTokenIssuer::new(
                authority,
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
        let (header, body) = request.into_parts();
        trace! {
            target: "enrollment_token_issuer",
            from   = %from,
            id     = %header.id(),
            method = ?header.method(),
            path   = %header.path(),
            body   = %header.has_body(),
            "request"
        }
        let res = match (header.method(), header.path()) {
            (Some(Method::Post), "/") | (Some(Method::Post), "/tokens") => {
                let att: CreateToken = CreateToken::decode(&body.unwrap_or_default())?;
                let duration = att.ttl_secs().map(Duration::from_secs);
                let ttl_count = att.ttl_count();

                let res = self
                    .issuer
                    .issue_token(&from, att.into_owned_attributes(), duration, ttl_count)
                    .await?;

                match res {
                    Either::Left(otc) => Response::ok()
                        .with_headers(&header)
                        .body(otc)
                        .encode_body()?,
                    Either::Right(error) => Response::forbidden(&header, &error.0).encode_body()?,
                }
            }
            _ => Response::unknown_path(&header).encode_body()?,
        };
        c.send(return_route, res).await
    }
}
