use minicbor::Decoder;
use ockam::identity::secure_channel_required;
use ockam::identity::OneTimeCode;
use ockam::identity::{Identifier, IdentitySecureChannelLocalInfo};
use ockam_core::api::{Method, Request, Response};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Result, Routed, Worker};
use ockam_node::Context;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::trace;

use crate::authenticator::direct::types::CreateToken;
use crate::authenticator::enrollment_tokens::authenticator::MAX_TOKEN_DURATION;
use crate::authenticator::enrollment_tokens::types::Token;
use crate::authenticator::enrollment_tokens::EnrollmentTokenAuthenticator;

pub struct EnrollmentTokenIssuer(pub(super) EnrollmentTokenAuthenticator);

impl EnrollmentTokenIssuer {
    async fn issue_token(
        &self,
        enroller: &Identifier,
        attrs: HashMap<String, String>,
        token_duration: Option<Duration>,
    ) -> Result<OneTimeCode> {
        let otc = OneTimeCode::new();
        let max_token_duration = token_duration.unwrap_or(MAX_TOKEN_DURATION);
        let tkn = Token {
            attrs,
            generated_by: enroller.clone(),
            time: Instant::now(),
            max_token_duration,
        };
        self.0
            .tokens
            .write()
            .map(|mut r| {
                r.put(*otc.code(), tkn);
                otc
            })
            .map_err(|_| {
                ockam_core::Error::new(
                    Origin::Other,
                    Kind::Internal,
                    "failed to get read lock on tokens table",
                )
            })
    }
}

#[ockam_core::worker]
impl Worker for EnrollmentTokenIssuer {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let from = i.their_identity_id();
            let mut dec = Decoder::new(m.as_body());
            let req: Request = dec.decode()?;
            trace! {
                target: "ockam_api::authenticator::direct::enrollment_token_issuer",
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
                    let duration = att.token_duration();
                    match self
                        .issue_token(&from, att.into_owned_attributes(), duration)
                        .await
                    {
                        Ok(otc) => Response::ok(req.id()).body(&otc).to_vec()?,
                        Err(error) => {
                            ockam_core::api::internal_error(&req, &error.to_string()).to_vec()?
                        }
                    }
                }
                _ => ockam_core::api::unknown_path(&req).to_vec()?,
            };
            c.send(m.return_route(), res).await
        } else {
            secure_channel_required(c, m).await
        }
    }
}
