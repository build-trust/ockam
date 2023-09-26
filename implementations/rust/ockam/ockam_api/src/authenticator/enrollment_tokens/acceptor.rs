use minicbor::Decoder;
use ockam::identity::utils::now;
use ockam::identity::IdentitySecureChannelLocalInfo;
use ockam::identity::OneTimeCode;
use ockam::identity::{secure_channel_required, TRUST_CONTEXT_ID};
use ockam::identity::{AttributesEntry, IdentityAttributesWriter};
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{Result, Routed, Worker};
use ockam_node::Context;
use tracing::trace;

use crate::authenticator::enrollment_tokens::EnrollmentTokenAuthenticator;

pub struct EnrollmentTokenAcceptor(
    pub(super) EnrollmentTokenAuthenticator,
    pub(super) Arc<dyn IdentityAttributesWriter>,
);

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
                    //TODO: move out of the worker handle_message implementation
                    let otc: OneTimeCode = dec.decode()?;
                    let token = match self.0.tokens.write() {
                        Ok(mut r) => {
                            if let Some(tkn) = r.pop(otc.code()) {
                                if tkn.time.elapsed() > tkn.max_token_duration {
                                    Err(Response::forbidden(&req, "expired token"))
                                } else {
                                    Ok(tkn)
                                }
                            } else {
                                Err(Response::forbidden(&req, "unknown token"))
                            }
                        }
                        Err(_) => Err(Response::internal_error(
                            &req,
                            "Failed to get read lock on tokens table",
                        )),
                    };
                    match token {
                        Ok(tkn) => {
                            //TODO: fixme:  unify use of hashmap vs btreemap
                            let trust_context = self.0.trust_context.as_bytes().to_vec();
                            let attrs = tkn
                                .attrs
                                .iter()
                                .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
                                .chain([(TRUST_CONTEXT_ID.to_owned(), trust_context)].into_iter())
                                .collect();
                            let entry =
                                AttributesEntry::new(attrs, now()?, None, Some(tkn.generated_by));
                            self.1.put_attributes(&from, entry).await?;
                            Response::ok(&req).to_vec()?
                        }
                        Err(err) => err.to_vec()?,
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
