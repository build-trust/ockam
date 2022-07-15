pub mod types;

use crate::signer;
use crate::util::response;
use crate::{Error, Method, Request, Response, ResponseBuilder};
use core::marker::PhantomData;
use minicbor::Decoder;
use ockam_core::{self, Result, Routed, Worker};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::{IdentityIdentifier, IdentitySecureChannelLocalInfo};
use ockam_node::Context;
use tracing::{trace, warn};
use types::{AddEnroller, AddMember, Placeholder};

// storage scopes:
const ENROLLER: &str = "enroller";
const MEMBER: &str = "member";

#[derive(Debug)]
pub struct Server<M, S> {
    store: S,
    signer: signer::Client,
    _mode: PhantomData<fn() -> M>,
}

/// Marker type, used for privileged API operations.
#[derive(Debug)]
pub enum Admin {}

/// Marker type, used for unprivileged API operations.
#[derive(Debug)]
pub enum General {}

#[ockam_core::worker]
impl<S: AuthenticatedStorage> Worker for Server<General, S> {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        let i = IdentitySecureChannelLocalInfo::find_info(m.local_message())?;
        let r = self.on_request(i.their_identity_id(), m.as_body()).await?;
        c.send(m.return_route(), r).await
    }
}

#[ockam_core::worker]
impl<S: AuthenticatedStorage> Worker for Server<Admin, S> {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        let r = self.on_admin_request(m.as_body()).await?;
        c.send(m.return_route(), r).await
    }
}

impl<S: AuthenticatedStorage> Server<General, S> {
    pub fn new(store: S, signer: signer::Client) -> Self {
        Server {
            store,
            signer,
            _mode: PhantomData,
        }
    }

    async fn on_request(&mut self, from: &IdentityIdentifier, data: &[u8]) -> Result<Vec<u8>> {
        let mut dec = Decoder::new(data);
        let req: Request = dec.decode()?;

        trace! {
            target: "ockam_api::authenticator::direct::server",
            from   = %from,
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        let res = match req.method() {
            Some(Method::Post) => match req.path_segments::<2>().as_slice() {
                // Enroller wants to add a member.
                ["member"] => {
                    if let Some(err) = self.check_enroller_auth(&req, from.key_id()).await? {
                        err.to_vec()?
                    } else {
                        let add: AddMember = dec.decode()?;
                        let member = minicbor::to_vec(Placeholder)?;
                        self.store
                            .set(MEMBER, add.member().as_str().to_string(), member)
                            .await?;
                        Response::ok(req.id()).to_vec()?
                    }
                }
                // Member wants a credential.
                ["credential"] => {
                    if let Some(err) = self.check_member_auth(&req, from.key_id()).await? {
                        err.to_vec()?
                    } else {
                        let crd = self.signer.sign_id(from).await?;
                        Response::ok(req.id()).body(crd).to_vec()?
                    }
                }
                _ => response::unknown_path(&req).to_vec()?,
            },
            Some(Method::Get) => match req.path_segments::<3>().as_slice() {
                // Enroller checks member data.
                ["member", id] => {
                    if let Some(err) = self.check_enroller_auth(&req, from.key_id()).await? {
                        err.to_vec()?
                    } else if let Some(data) = self.store.get(MEMBER, id).await? {
                        let member = minicbor::decode::<Placeholder>(&data)?;
                        Response::ok(req.id()).body(member).to_vec()?
                    } else {
                        Response::not_found(req.id()).to_vec()?
                    }
                }
                _ => response::unknown_path(&req).to_vec()?,
            },
            _ => response::invalid_method(&req).to_vec()?,
        };

        Ok(res)
    }

    async fn check_enroller_auth<'a>(
        &self,
        req: &'a Request<'_>,
        enroller: &str,
    ) -> Result<Option<ResponseBuilder<Error<'a>>>> {
        if let Some(data) = self.store.get(ENROLLER, enroller).await? {
            minicbor::decode::<Placeholder>(&data)?;
            return Ok(None);
        }
        warn! {
            target: "ockam_api::authenticator::direct::server",
            enroller = %enroller,
            id       = %req.id(),
            method   = ?req.method(),
            path     = %req.path(),
            body     = %req.has_body(),
            "unauthorised enroller"
        }
        Ok(Some(response::forbidden(req, "unauthorized enroller")))
    }

    async fn check_member_auth<'a>(
        &self,
        req: &'a Request<'_>,
        member: &str,
    ) -> Result<Option<ResponseBuilder<Error<'a>>>> {
        if let Some(data) = self.store.get(MEMBER, member).await? {
            minicbor::decode::<Placeholder>(&data)?;
            return Ok(None);
        }
        warn! {
            target: "ockam_api::authenticator::direct::server",
            member   = %member,
            id       = %req.id(),
            method   = ?req.method(),
            path     = %req.path(),
            body     = %req.has_body(),
            "unauthorised member"
        }
        Ok(Some(response::forbidden(req, "unauthorized member")))
    }
}

impl<S: AuthenticatedStorage> Server<Admin, S> {
    pub fn admin(store: S, signer: signer::Client) -> Self {
        Server {
            store,
            signer,
            _mode: PhantomData,
        }
    }

    async fn on_admin_request(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut dec = Decoder::new(data);
        let req: Request = dec.decode()?;

        debug! {
            target: "ockam_api::authenticator::direct::server",
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "unauthenticated request"
        }

        let res = match req.method() {
            Some(Method::Post) => match req.path_segments::<2>().as_slice() {
                // Admin wants to add an enroller.
                ["enroller"] => {
                    let add: AddEnroller = dec.decode()?;
                    let enroller = minicbor::to_vec(Placeholder)?;
                    self.store
                        .set(ENROLLER, add.enroller().as_str().to_string(), enroller)
                        .await?;
                    Response::ok(req.id()).to_vec()?
                }
                _ => response::unknown_path(&req).to_vec()?,
            },
            Some(Method::Get) => match req.path_segments::<3>().as_slice() {
                // Admin wants to check enroller data.
                ["enroller", id] => {
                    if let Some(data) = self.store.get(ENROLLER, id).await? {
                        let enroller = minicbor::decode::<Placeholder>(&data)?;
                        Response::ok(req.id()).body(enroller).to_vec()?
                    } else {
                        Response::not_found(req.id()).to_vec()?
                    }
                }
                _ => response::unknown_path(&req).to_vec()?,
            },
            Some(Method::Delete) => match req.path_segments::<3>().as_slice() {
                // Admin wants to remove an enroller.
                ["enroller", id] => {
                    self.store.del(ENROLLER, id).await?;
                    Response::ok(req.id()).to_vec()?
                }
                _ => response::unknown_path(&req).to_vec()?,
            },
            _ => response::invalid_method(&req).to_vec()?,
        };

        Ok(res)
    }
}
