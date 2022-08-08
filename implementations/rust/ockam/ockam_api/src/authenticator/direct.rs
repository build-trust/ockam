pub mod types;

use crate::auth::types::Attributes;
use crate::signer::types::{Credential, IdentityId};
use crate::util::response;
use crate::{assert_request_match, assert_response_match, Cbor};
use crate::{signer, Timestamp};
use crate::{Error, Method, Request, RequestBuilder, Response, ResponseBuilder, Status};
use core::time::Duration;
use core::{fmt, str};
use minicbor::encode::write::Cursor;
use minicbor::{Decoder, Encode};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{self, Address, Result, Route, Routed, Worker};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::{IdentityIdentifier, IdentitySecureChannelLocalInfo};
use ockam_node::Context;
use tracing::{trace, warn};
use types::{AddEnroller, AddMember, Placeholder};

// storage scopes:
const ENROLLER: &str = "enroller";
const MEMBER: &str = "member";

const MAX_VALIDITY: Duration = Duration::from_secs(2 * 3600);

#[derive(Debug)]
pub struct Server<S, T> {
    m_store: S, // member store
    e_store: T, // enroller store
    signer: signer::Client,
    admin: Option<IdentityId<'static>>,
}

#[ockam_core::worker]
impl<S, T> Worker for Server<S, T>
where
    S: AuthenticatedStorage,
    T: AuthenticatedStorage,
{
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let r = self.on_request(i.their_identity_id(), m.as_body()).await?;
            c.send(m.return_route(), r).await
        } else {
            let mut dec = Decoder::new(m.as_body());
            let req: Request = dec.decode()?;
            let res = response::forbidden(&req, "secure channel required").to_vec()?;
            c.send(m.return_route(), res).await
        }
    }
}

impl<S, T> Server<S, T>
where
    S: AuthenticatedStorage,
    T: AuthenticatedStorage,
{
    pub fn new(m_store: S, e_store: T, signer: signer::Client) -> Self {
        Server {
            m_store,
            e_store,
            signer,
            admin: None,
        }
    }

    pub fn set_admin(&mut self, id: &IdentityIdentifier) {
        self.admin = Some(IdentityId::new(id.key_id().to_string()))
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
                // Admin wants to add an enroller.
                ["enroller"] if self.is_admin(from) => {
                    let add: AddEnroller = dec.decode()?;
                    let enroller = minicbor::to_vec(Placeholder)?;
                    self.e_store
                        .set(ENROLLER, add.enroller().as_str().to_string(), enroller)
                        .await?;
                    Response::ok(req.id()).to_vec()?
                }
                ["enroller"] => Response::unauthorized(req.id()).to_vec()?,
                // Enroller wants to add a member.
                ["member"] => {
                    if let Some(err) = check_enroller(&self.e_store, &req, from.key_id()).await? {
                        err.to_vec()?
                    } else {
                        let add: AddMember = dec.decode()?;
                        let member = minicbor::to_vec(Placeholder)?;
                        self.m_store
                            .set(MEMBER, add.member().as_str().to_string(), member)
                            .await?;
                        Response::ok(req.id()).to_vec()?
                    }
                }
                // Member wants a credential.
                ["credential"] => {
                    if let Some(err) = check_member(&self.m_store, &req, from.key_id()).await? {
                        err.to_vec()?
                    } else {
                        let mut attrs = Attributes::new();
                        attrs.put("id", from.key_id().as_bytes());
                        let ts = Timestamp::now().ok_or_else(invalid_sys_time)?;
                        let mut timestamp = Cursor::new([0; 9]);
                        minicbor::encode(ts, &mut timestamp)?;
                        attrs.put("ts", &timestamp.get_ref()[..timestamp.position()]);
                        let crd = self.signer.sign(&attrs).await?;
                        Response::ok(req.id()).body(crd).to_vec()?
                    }
                }
                _ => response::unknown_path(&req).to_vec()?,
            },
            Some(Method::Get) => match req.path_segments::<3>().as_slice() {
                // Admin wants to check enroller data.
                ["enroller", id] if self.is_admin(from) => {
                    if let Some(data) = self.e_store.get(ENROLLER, id).await? {
                        let enroller = minicbor::decode::<Placeholder>(&data)?;
                        Response::ok(req.id()).body(enroller).to_vec()?
                    } else {
                        Response::not_found(req.id()).to_vec()?
                    }
                }
                ["enroller", _] => Response::unauthorized(req.id()).to_vec()?,
                // Enroller checks member data.
                ["member", id] => {
                    if let Some(err) = check_enroller(&self.e_store, &req, from.key_id()).await? {
                        err.to_vec()?
                    } else if let Some(data) = self.m_store.get(MEMBER, id).await? {
                        let member = minicbor::decode::<Placeholder>(&data)?;
                        Response::ok(req.id()).body(member).to_vec()?
                    } else {
                        Response::not_found(req.id()).to_vec()?
                    }
                }
                // Validate member credential.
                ["credential"] => {
                    let crd: Credential = dec.decode()?;
                    if let Ok(att) = self.signer.verify(&crd).await {
                        if let Some(err) = check_credential(&self.m_store, &req, &att).await? {
                            err.to_vec()?
                        } else {
                            Response::ok(req.id())
                                .body(Cbor(crd.attributes()))
                                .to_vec()?
                        }
                    } else {
                        Response::unauthorized(req.id()).to_vec()?
                    }
                }
                _ => response::unknown_path(&req).to_vec()?,
            },
            Some(Method::Delete) => match req.path_segments::<3>().as_slice() {
                // Admin wants to remove an enroller.
                ["enroller", id] if self.is_admin(from) => {
                    self.e_store.del(ENROLLER, id).await?;
                    Response::ok(req.id()).to_vec()?
                }
                ["enroller", _] => Response::unauthorized(req.id()).to_vec()?,
                _ => response::unknown_path(&req).to_vec()?,
            },
            _ => response::invalid_method(&req).to_vec()?,
        };

        Ok(res)
    }

    /// Check that the given identity ID matches the one configured as admin.
    fn is_admin(&self, k: &IdentityIdentifier) -> bool {
        Some(k.key_id().as_str()) == self.admin.as_ref().map(|a| a.as_str())
    }
}

async fn check_credential<'a, S: AuthenticatedStorage>(
    store: &S,
    req: &'a Request<'_>,
    attrs: &Attributes<'_>,
) -> Result<Option<ResponseBuilder<Error<'a>>>> {
    // Verify that the credential has not expired

    let ts = if let Some(val) = attrs.get("ts") {
        minicbor::decode(val)?
    } else {
        return Ok(Some(response::bad_request(
            req,
            "missing credential timestamp",
        )));
    };
    let now = Timestamp::now().ok_or_else(invalid_sys_time)?;
    if let Some(dur) = now.elapsed(ts) {
        if dur > MAX_VALIDITY {
            return Ok(Some(response::forbidden(req, "credential expired")));
        }
    } else {
        return Ok(Some(response::bad_request(
            req,
            "invalid credential timestamp",
        )));
    }

    // Verify that the member is authorised

    let id = if let Some(val) = attrs.get("id") {
        str::from_utf8(val).map_err(invalid_utf8)?
    } else {
        return Ok(Some(response::bad_request(
            req,
            "missing credential identity",
        )));
    };
    check_member(store, req, id).await
}

async fn check_member<'a, S: AuthenticatedStorage>(
    store: &S,
    req: &'a Request<'_>,
    member: &str,
) -> Result<Option<ResponseBuilder<Error<'a>>>> {
    if let Some(data) = store.get(MEMBER, member).await? {
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

async fn check_enroller<'a, S: AuthenticatedStorage>(
    store: &S,
    req: &'a Request<'_>,
    enroller: &str,
) -> Result<Option<ResponseBuilder<Error<'a>>>> {
    if let Some(data) = store.get(ENROLLER, enroller).await? {
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

fn invalid_sys_time() -> ockam_core::Error {
    ockam_core::Error::new(Origin::Node, Kind::Internal, "invalid system time")
}

fn invalid_utf8(e: str::Utf8Error) -> ockam_core::Error {
    ockam_core::Error::new(Origin::Application, Kind::Invalid, e)
}

pub struct Client {
    ctx: Context,
    route: Route,
    buf: Vec<u8>,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("route", &self.route)
            .finish()
    }
}

impl Client {
    pub async fn new(r: Route, ctx: &Context) -> Result<Self> {
        let ctx = ctx.new_detached(Address::random_local()).await?;
        Ok(Client {
            ctx,
            route: r,
            buf: Vec::new(),
        })
    }

    pub async fn add_enroller(&mut self, id: IdentityId<'_>) -> Result<()> {
        let req = Request::post("/enroller").body(AddEnroller::new(id));
        self.buf = self.request("add-enroller", "add_enroller", &req).await?;
        assert_response_match(None, &self.buf);
        let mut d = Decoder::new(&self.buf);
        let res = response("add-enroller", &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error("add-enroller", &res, &mut d))
        }
    }

    pub async fn add_member(&mut self, id: IdentityId<'_>) -> Result<()> {
        let req = Request::post("/member").body(AddMember::new(id));
        self.buf = self.request("add-member", "add_member", &req).await?;
        assert_response_match(None, &self.buf);
        let mut d = Decoder::new(&self.buf);
        let res = response("add-member", &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error("add-member", &res, &mut d))
        }
    }

    pub async fn credential(&mut self) -> Result<Credential<'_>> {
        let req = Request::post("/credential");
        self.buf = self.request("new-credential", None, &req).await?;
        assert_response_match("credential", &self.buf);
        let mut d = Decoder::new(&self.buf);
        let res = response("new-credential", &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(d.decode()?)
        } else {
            Err(error("new-credential", &res, &mut d))
        }
    }

    pub async fn validate(&mut self, c: &Credential<'_>) -> Result<Attributes<'_>> {
        let req = Request::get("/credential").body(c);
        self.buf = self
            .request("verify-credential", "credential", &req)
            .await?;
        assert_response_match("attributes", &self.buf);
        let mut d = Decoder::new(&self.buf);
        let res = response("verify-credential", &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(d.decode()?)
        } else {
            Err(error("verify-credential", &res, &mut d))
        }
    }

    /// Encode request header and body (if any) and send the package to the server.
    async fn request<T>(
        &mut self,
        label: &str,
        schema: impl Into<Option<&str>>,
        req: &RequestBuilder<'_, T>,
    ) -> Result<Vec<u8>>
    where
        T: Encode<()>,
    {
        let mut buf = Vec::new();
        req.encode(&mut buf)?;
        assert_request_match(schema, &buf);
        trace! {
            target: "ockam_api::authenticator::direct::client",
            id     = %req.header().id(),
            method = ?req.header().method(),
            path   = %req.header().path(),
            body   = %req.header().has_body(),
            "-> {label}"
        };
        let vec: Vec<u8> = self.ctx.send_and_receive(self.route.clone(), buf).await?;
        Ok(vec)
    }
}

/// Decode and log response header.
fn response(label: &str, dec: &mut Decoder<'_>) -> Result<Response> {
    let res: Response = dec.decode()?;
    trace! {
        target: "ockam_api::authenticator::direct::client",
        re     = %res.re(),
        id     = %res.id(),
        status = ?res.status(),
        body   = %res.has_body(),
        "<- {label}"
    }
    Ok(res)
}

/// Decode, log and map response error to ockam_core error.
fn error(label: &str, res: &Response, dec: &mut Decoder<'_>) -> ockam_core::Error {
    if res.has_body() {
        let err = match dec.decode::<Error>() {
            Ok(e) => e,
            Err(e) => return e.into(),
        };
        warn! {
            target: "ockam_api::authenticator::direct::client",
            id     = %res.id(),
            re     = %res.re(),
            status = ?res.status(),
            error  = ?err.message(),
            "<- {label}"
        }
        let msg = err.message().unwrap_or(label);
        ockam_core::Error::new(Origin::Application, Kind::Protocol, msg)
    } else {
        ockam_core::Error::new(Origin::Application, Kind::Protocol, label)
    }
}
