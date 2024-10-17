use either::Either;
use minicbor::Decoder;
use tracing::trace;

use ockam::identity::{Identifier, IdentitiesAttributes};
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{Result, Routed, SecureChannelLocalInfo, Worker};
use ockam_node::Context;

use crate::authenticator::direct::types::AddMember;
use crate::authenticator::direct::DirectAuthenticator;
use crate::authenticator::AuthorityMembersRepository;

use super::AccountAuthorityInfo;

pub struct DirectAuthenticatorWorker {
    authenticator: DirectAuthenticator,
}

impl DirectAuthenticatorWorker {
    pub fn new(
        members: Arc<dyn AuthorityMembersRepository>,
        identities_attributes: Arc<IdentitiesAttributes>,
        account_authority: Option<AccountAuthorityInfo>,
    ) -> Self {
        Self {
            authenticator: DirectAuthenticator::new(
                members,
                identities_attributes,
                account_authority,
            ),
        }
    }
}

#[ockam_core::worker]
impl Worker for DirectAuthenticatorWorker {
    type Message = Vec<u8>;
    type Context = Context;

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
            target: "direct_authenticator",
            from   = %from,
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }
        let path_segments = req.path_segments::<5>();
        let res = match (req.method(), path_segments.as_slice()) {
            (Some(Method::Post), [""]) | (Some(Method::Post), ["members"]) => {
                let add: AddMember = dec.decode()?;
                let res = self
                    .authenticator
                    .add_member(&from, add.member(), add.attributes())
                    .await?;
                match res {
                    Either::Left(_) => Response::ok().with_headers(&req).to_vec()?,
                    Either::Right(error) => Response::forbidden(&req, &error.0).to_vec()?,
                }
            }
            (Some(Method::Get), ["member_ids"]) => {
                let res = self.authenticator.list_members(&from).await?;
                match res {
                    Either::Left(entries) => {
                        let ids: Vec<Identifier> = entries.into_keys().collect();
                        Response::ok().with_headers(&req).body(ids).to_vec()?
                    }
                    Either::Right(error) => Response::forbidden(&req, &error.0).to_vec()?,
                }
            }
            (Some(Method::Get), [""]) | (Some(Method::Get), ["members"]) => {
                let res = self.authenticator.list_members(&from).await?;

                match res {
                    Either::Left(entries) => {
                        Response::ok().with_headers(&req).body(entries).to_vec()?
                    }
                    Either::Right(error) => Response::forbidden(&req, &error.0).to_vec()?,
                }
            }
            (Some(Method::Get), [id]) | (Some(Method::Get), ["members", id]) => {
                let identifier = Identifier::try_from(id.to_string())?;
                let res = self.authenticator.show_member(&from, &identifier).await?;

                match res {
                    Either::Left(body) => Response::ok().with_headers(&req).body(body).to_vec()?,
                    Either::Right(error) => Response::forbidden(&req, &error.0).to_vec()?,
                }
            }
            (Some(Method::Delete), ["members"]) => {
                let res = self.authenticator.delete_all_members(&from).await?;
                match res {
                    Either::Left(_) => Response::ok().with_headers(&req).to_vec()?,
                    Either::Right(error) => Response::forbidden(&req, &error.0).to_vec()?,
                }
            }
            (Some(Method::Delete), [id]) | (Some(Method::Delete), ["members", id]) => {
                let identifier = Identifier::try_from(id.to_string())?;
                let res = self.authenticator.delete_member(&from, &identifier).await?;

                match res {
                    Either::Left(_) => Response::ok().with_headers(&req).to_vec()?,
                    Either::Right(error) => Response::forbidden(&req, &error.0).to_vec()?,
                }
            }
            _ => Response::unknown_path(&req).to_vec()?,
        };

        c.send(return_route, res).await?;

        Ok(())
    }
}
