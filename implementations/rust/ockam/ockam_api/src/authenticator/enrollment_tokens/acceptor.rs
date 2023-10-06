use minicbor::Decoder;
use ockam::identity::utils::now;
use ockam::identity::Identifier;
use ockam::identity::IdentitySecureChannelLocalInfo;
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{Result, Routed, Worker};
use ockam_node::Context;
use tracing::trace;

use crate::authenticator::enrollment_tokens::EnrollmentTokenAuthenticator;
use crate::authenticator::one_time_code::OneTimeCode;
use crate::authenticator::{secure_channel_required, Member, MembersStorage};

pub struct EnrollmentTokenAcceptor {
    pub(super) authenticator: EnrollmentTokenAuthenticator,
    pub(super) members_storage: Arc<dyn MembersStorage>,
}

impl EnrollmentTokenAcceptor {
    pub fn new(
        authenticator: EnrollmentTokenAuthenticator,
        members_storage: Arc<dyn MembersStorage>,
    ) -> Self {
        Self {
            authenticator,
            members_storage,
        }
    }
}

impl EnrollmentTokenAcceptor {
    async fn accept_token(
        &mut self,
        req: &RequestHeader,
        otc: OneTimeCode,
        from: &Identifier,
    ) -> Result<Vec<u8>> {
        let token = {
            let mut tokens = match self.authenticator.tokens.write() {
                Ok(tokens) => tokens,
                Err(_) => {
                    return Ok(Response::internal_error(
                        req,
                        "Failed to get read lock on tokens table",
                    )
                    .to_vec()?);
                }
            };

            let token = if let Some(token) = tokens.remove(otc.code()) {
                if token.created_at.elapsed() > token.ttl {
                    return Ok(Response::forbidden(req, "expired token").to_vec()?);
                } else {
                    token
                }
            } else {
                return Ok(Response::forbidden(req, "unknown token").to_vec()?);
            };

            if token.ttl_count > 1 {
                let mut token_clone = token.clone();
                token_clone.ttl_count -= 1;
                tokens.insert(*otc.code(), token_clone);
            }

            token
        };

        //TODO: fixme:  unify use of hashmap vs btreemap
        let attrs = token
            .attrs
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .collect();

        let member = Member::new(
            from.clone(),
            attrs,
            Some(token.issued_by),
            now().unwrap(),
            false,
        );

        if let Err(_err) = self.members_storage.add_member(member).await {
            return Ok(Response::internal_error(req, "members storage error").to_vec()?);
        }

        Ok(Response::ok(req).to_vec()?)
    }
}

#[ockam_core::worker]
impl Worker for EnrollmentTokenAcceptor {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let from = i.their_identity_id();
            let mut dec = Decoder::new(m.as_body());
            let req: RequestHeader = dec.decode()?;
            trace! {
                target: "ockam_api::authenticator::direct::enrollment_token_acceptor",
                from   = %from,
                id     = %req.id(),
                method = ?req.method(),
                path   = %req.path(),
                body   = %req.has_body(),
                "request"
            }
            let res = match (req.method(), req.path()) {
                (Some(Method::Post), "/") | (Some(Method::Post), "/credential") => {
                    let otc: OneTimeCode = dec.decode()?;
                    self.accept_token(&req, otc, &from).await?
                }
                _ => Response::unknown_path(&req).to_vec()?,
            };
            c.send(m.return_route(), res).await
        } else {
            secure_channel_required(c, m).await
        }
    }
}
