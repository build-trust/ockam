use crate::authenticator::enrollment_tokens::EnrollmentTokenAcceptor;
use crate::authenticator::one_time_code::OneTimeCode;
use crate::authenticator::{AuthorityEnrollmentTokenRepository, AuthorityMembersRepository};
use either::Either;
use ockam::identity::Identifier;
use ockam_core::api::{Method, Request, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::identity::SecureChannelLocalInfo;
use ockam_core::{Decodable, Result, Routed, Worker};
use ockam_node::Context;
use tracing::trace;

pub struct EnrollmentTokenAcceptorWorker {
    pub(super) acceptor: EnrollmentTokenAcceptor,
}

impl EnrollmentTokenAcceptorWorker {
    pub fn new(
        authority: &Identifier,
        tokens: Arc<dyn AuthorityEnrollmentTokenRepository>,
        members: Arc<dyn AuthorityMembersRepository>,
    ) -> Self {
        Self {
            acceptor: EnrollmentTokenAcceptor::new(authority, tokens, members),
        }
    }
}

#[ockam_core::worker]
impl Worker for EnrollmentTokenAcceptorWorker {
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
            target: "enrollment_token_acceptor",
            from   = %from,
            id     = %header.id(),
            method = ?header.method(),
            path   = %header.path(),
            body   = %header.has_body(),
            "request"
        }
        let res = match (header.method(), header.path()) {
            (Some(Method::Post), "/") | (Some(Method::Post), "/credential") => {
                let otc = OneTimeCode::decode(&body.unwrap_or_default())?;
                let res = self.acceptor.accept_token(otc, &from).await?;
                match res {
                    Either::Left(_) => Response::ok().with_headers(&header).encode_body()?,
                    Either::Right(error) => Response::forbidden(&header, &error.0).encode_body()?,
                }
            }
            _ => Response::unknown_path(&header).encode_body()?,
        };
        c.send(return_route, res).await
    }
}
