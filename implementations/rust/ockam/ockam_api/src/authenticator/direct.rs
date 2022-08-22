pub mod types;

use core::{fmt, str};
use minicbor::{Decoder, Encode};
use ockam_core::api::{self, assert_request_match, assert_response_match};
use ockam_core::api::{Error, Method, Request, RequestBuilder, Response, ResponseBuilder, Status};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{self, Address, Result, Route, Routed, Worker};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::credential::{Credential, SchemaId};
use ockam_identity::{Identity, IdentityIdentifier, IdentitySecureChannelLocalInfo, IdentityVault};
use ockam_node::Context;
use serde_json as json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{trace, warn};
use types::AddMember;

use self::types::Enroller;

const MEMBER: &str = "member";

/// Schema identifier for a project membership credential.
///
/// The credential will consist of the following attributes:
///
/// - `project_id` : bytes
/// - `role`: b"member"
pub const PROJECT_MEMBER_SCHEMA: SchemaId = SchemaId(1);
pub const PROJECT_ID: &str = "project_id";
pub const ROLE: &str = "role";

pub struct Server<S, V: IdentityVault> {
    project: Vec<u8>,
    store: S,
    ident: Identity<V>,
    epath: PathBuf,
    enrollers: HashMap<IdentityIdentifier, Enroller>,
}

#[ockam_core::worker]
impl<S, V> Worker for Server<S, V>
where
    S: AuthenticatedStorage,
    V: IdentityVault,
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
            let res = api::forbidden(&req, "secure channel required").to_vec()?;
            c.send(m.return_route(), res).await
        }
    }
}

impl<S, V> Server<S, V>
where
    S: AuthenticatedStorage,
    V: IdentityVault,
{
    pub fn new<P>(project: Vec<u8>, store: S, enrollers: P, identity: Identity<V>) -> Self
    where
        P: AsRef<Path>,
    {
        Server {
            project,
            store,
            ident: identity,
            epath: enrollers.as_ref().to_path_buf(),
            enrollers: HashMap::new(),
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
                ["members"] => match self.check_enroller(&req, from).await {
                    Ok(None) => {
                        let add: AddMember = dec.decode()?;
                        let tru = minicbor::to_vec(true)?;
                        self.store
                            .set(add.member().key_id(), MEMBER.to_string(), tru)
                            .await?;
                        Response::ok(req.id()).to_vec()?
                    }
                    Ok(Some(e)) => e.to_vec()?,
                    Err(error) => api::internal_error(&req, &error.to_string()).to_vec()?,
                },
                // Member wants a credential.
                ["credential"] => match self.check_member(&req, from).await {
                    Ok(None) => {
                        let crd = Credential::builder(from.clone())
                            .with_schema(PROJECT_MEMBER_SCHEMA)
                            .with_attribute(PROJECT_ID, &self.project)
                            .with_attribute(ROLE, b"member");

                        let crd = self.ident.issue_credential(crd).await?;
                        Response::ok(req.id()).body(crd).to_vec()?
                    }
                    Ok(Some(e)) => e.to_vec()?,
                    Err(error) => api::internal_error(&req, &error.to_string()).to_vec()?,
                },
                _ => api::unknown_path(&req).to_vec()?,
            },
            _ => api::invalid_method(&req).to_vec()?,
        };

        Ok(res)
    }

    async fn check_enroller<'a>(
        &mut self,
        req: &'a Request<'_>,
        enroller: &IdentityIdentifier,
    ) -> Result<Option<ResponseBuilder<Error<'a>>>> {
        let contents = std::fs::read_to_string(&self.epath)
            .map_err(|e| ockam_core::Error::new(Origin::Other, Kind::Io, e))?;

        let enrollers: HashMap<IdentityIdentifier, Enroller> = json::from_str(&contents)
            .map_err(|e| ockam_core::Error::new(Origin::Other, Kind::Invalid, e))?;

        self.enrollers = enrollers;

        if self.enrollers.contains_key(enroller) {
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

        Ok(Some(api::forbidden(req, "unauthorized enroller")))
    }

    async fn check_member<'a>(
        &self,
        req: &'a Request<'_>,
        member: &IdentityIdentifier,
    ) -> Result<Option<ResponseBuilder<Error<'a>>>> {
        if let Some(data) = self.store.get(member.key_id(), MEMBER).await? {
            if minicbor::decode(&data)? {
                return Ok(None);
            }
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

        Ok(Some(api::forbidden(req, "unauthorized member")))
    }
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

    pub async fn add_member(&mut self, id: IdentityIdentifier) -> Result<()> {
        let req = Request::post("/members").body(AddMember::new(id));
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
