use crate::authenticator::enrollment_tokens::EnrollmentTokenAcceptor;
use crate::authenticator::one_time_code::OneTimeCode;
use crate::authenticator::{AuthorityEnrollmentTokenRepository, AuthorityMembersRepository};
use either::Either;
use minicbor::Decoder;
use ockam::identity::Identifier;
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{Result, Routed, SecureChannelLocalInfo, Worker};
use ockam_node::Context;
use tracing::trace;

pub struct EnrollmentTokenAcceptorWorker {
    pub(super) acceptor: EnrollmentTokenAcceptor,
}

impl EnrollmentTokenAcceptorWorker {
    pub fn new(
        tokens: Arc<dyn AuthorityEnrollmentTokenRepository>,
        members: Arc<dyn AuthorityMembersRepository>,
    ) -> Self {
        Self {
            acceptor: EnrollmentTokenAcceptor::new(tokens, members),
        }
    }
}

#[ockam_core::worker]
impl Worker for EnrollmentTokenAcceptorWorker {
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
            target: "enrollment_token_acceptor",
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
                let res = self.acceptor.accept_token(otc, &from).await?;
                match res {
                    Either::Left(_) => Response::ok().with_headers(&req).to_vec()?,
                    Either::Right(error) => Response::forbidden(&req, &error.0).to_vec()?,
                }
            }
            _ => Response::unknown_path(&req).to_vec()?,
        };
        c.send(return_route, res).await
    }
}
