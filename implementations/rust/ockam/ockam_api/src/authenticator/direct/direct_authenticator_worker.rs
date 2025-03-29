use either::Either;
use tracing::trace;

use ockam::identity::models::IdentifierList;
use ockam::identity::{Identifier, IdentitiesAttributes};
use ockam_core::api::{Method, Request, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{Decodable, Result, Routed, SecureChannelLocalInfo, Worker};
use ockam_node::Context;

use crate::authenticator::direct::types::{AddMember, MemberList};
use crate::authenticator::direct::DirectAuthenticator;
use crate::authenticator::AuthorityMembersRepository;

use super::AccountAuthorityInfo;

pub struct DirectAuthenticatorWorker {
    authenticator: DirectAuthenticator,
}

impl DirectAuthenticatorWorker {
    pub fn new(
        authority: &Identifier,
        members: Arc<dyn AuthorityMembersRepository>,
        identities_attributes: Arc<IdentitiesAttributes>,
        account_authority: Option<AccountAuthorityInfo>,
    ) -> Self {
        Self {
            authenticator: DirectAuthenticator::new(
                authority,
                members,
                identities_attributes,
                account_authority,
            ),
        }
    }
}

#[ockam_core::worker]
impl Worker for DirectAuthenticatorWorker {
    type Message = Request<Vec<u8>>;
    type Context = Context;

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
            target: "direct_authenticator",
            from   = %from,
            id     = %header.id(),
            method = ?header.method(),
            path   = %header.path(),
            body   = %header.has_body(),
            "request"
        }
        let path_segments = header.path_segments::<5>();
        let res: Response<Vec<u8>> = match (header.method(), path_segments.as_slice()) {
            (Some(Method::Post), [""]) | (Some(Method::Post), ["members"]) => {
                let add = AddMember::decode(&body.unwrap_or_default())?;
                let res = self
                    .authenticator
                    .add_member(&from, add.member(), add.attributes())
                    .await?;
                match res {
                    Either::Left(_) => Response::ok().with_headers(&header).encode_body()?,
                    Either::Right(error) => Response::forbidden(&header, &error.0).encode_body()?,
                }
            }
            (Some(Method::Get), ["member_ids"]) => {
                let res = self.authenticator.list_members(&from).await?;
                match res {
                    Either::Left(entries) => {
                        let ids: Vec<Identifier> = entries.into_keys().collect();
                        Response::ok()
                            .with_headers(&header)
                            .body(IdentifierList(ids))
                            .encode_body()?
                    }
                    Either::Right(error) => Response::forbidden(&header, &error.0).encode_body()?,
                }
            }
            (Some(Method::Get), [""]) | (Some(Method::Get), ["members"]) => {
                let res = self.authenticator.list_members(&from).await?;

                match res {
                    Either::Left(entries) => Response::ok()
                        .with_headers(&header)
                        .body(MemberList(entries))
                        .encode_body()?,
                    Either::Right(error) => Response::forbidden(&header, &error.0).encode_body()?,
                }
            }
            (Some(Method::Get), [id]) | (Some(Method::Get), ["members", id]) => {
                let identifier = Identifier::try_from(id.to_string())?;
                let res = self.authenticator.show_member(&from, &identifier).await?;

                match res {
                    Either::Left(body) => Response::ok()
                        .with_headers(&header)
                        .body(body)
                        .encode_body()?,
                    Either::Right(error) => Response::forbidden(&header, &error.0).encode_body()?,
                }
            }
            (Some(Method::Delete), ["members"]) => {
                let res = self.authenticator.delete_all_members(&from).await?;
                match res {
                    Either::Left(_) => Response::ok().with_headers(&header).encode_body()?,
                    Either::Right(error) => Response::forbidden(&header, &error.0).encode_body()?,
                }
            }
            (Some(Method::Delete), [id]) | (Some(Method::Delete), ["members", id]) => {
                let identifier = Identifier::try_from(id.to_string())?;
                let res = self.authenticator.delete_member(&from, &identifier).await?;

                match res {
                    Either::Left(_) => Response::ok().with_headers(&header).encode_body()?,
                    Either::Right(error) => Response::forbidden(&header, &error.0).encode_body()?,
                }
            }
            _ => Response::unknown_path(&header).encode_body()?,
        };

        c.send(return_route, res).await?;

        Ok(())
    }
}
