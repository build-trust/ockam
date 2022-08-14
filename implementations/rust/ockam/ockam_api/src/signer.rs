pub mod types;

use crate::auth::types::Attributes;
use crate::signer::types::{AddIdentity, GetIdentityResponse};
use crate::util::response;
use crate::{assert_request_match, assert_response_match, Cbor};
use crate::{Error, Method, Request, RequestBuilder, Response, Status};
use core::fmt;
use minicbor::{Decoder, Encode};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::CowBytes;
use ockam_core::{self, vault, Address, Result, Route, Routed, Worker};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::change_history::IdentityChangeHistory;
use ockam_identity::{Identity, IdentityIdentifier, IdentityVault};
use ockam_node::Context;
use tracing::{trace, warn};
use types::{Credential, IdentityId, Signature};

pub struct Server<V: IdentityVault, S> {
    id: Identity<V>,
    storage: S,
}

impl<V: IdentityVault, S> fmt::Debug for Server<V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
            .field("id", self.id.identifier())
            .finish()
    }
}

#[ockam_core::worker]
impl<V: IdentityVault, S: AuthenticatedStorage> Worker for Server<V, S> {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        let r = self.on_request(m.as_body()).await?;
        c.send(m.return_route(), r).await
    }
}

impl<V: IdentityVault, S: AuthenticatedStorage> Server<V, S> {
    pub fn new(id: Identity<V>, storage: S) -> Self {
        Server { id, storage }
    }

    async fn on_request(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut dec = Decoder::new(data);
        let req: Request = dec.decode()?;

        trace! {
            target: "ockam_api::signer::server",
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        let res = match req.method() {
            Some(Method::Post) => match req.path_segments::<2>().as_slice() {
                ["sign"] => {
                    let pos = dec.position();
                    dec.decode::<Attributes>()?; // typecheck
                    let att = &dec.input()[pos..];
                    let iid = self.id.identifier();
                    let sig = self.id.create_signature(att).await?;
                    let bdy = {
                        let a = CowBytes::from(att);
                        let s = Signature::new(IdentityId::new(iid.key_id()), sig.as_ref());
                        Credential::new(a, s)
                    };
                    Response::ok(req.id()).body(bdy).to_vec()?
                }
                _ => response::unknown_path(&req).to_vec()?,
            },
            Some(Method::Get) => match req.path_segments::<2>().as_slice() {
                ["verify"] => {
                    let crd: Credential = dec.decode()?;
                    match self.verify(crd.attributes(), crd.signature()).await {
                        Ok(true) => Response::ok(req.id())
                            .body(Cbor(crd.attributes()))
                            .to_vec()?,
                        Ok(false) => Response::unauthorized(req.id()).to_vec()?,
                        Err(err) => {
                            warn! {
                                target: "ockam_api::signer::server",
                                id     = %req.id(),
                                method = ?req.method(),
                                path   = %req.path(),
                                body   = %req.has_body(),
                                error  = %err,
                                "signature verification failed"
                            }
                            Response::internal_error(req.id()).to_vec()?
                        }
                    }
                }
                ["identity"] => {
                    let k = self.id.export().await?;
                    let i = GetIdentityResponse::new(k.as_slice().into());
                    Response::ok(req.id()).body(i).to_vec()?
                }
                _ => response::unknown_path(&req).to_vec()?,
            },
            Some(Method::Put) => match req.path_segments::<2>().as_slice() {
                ["identity"] => {
                    let i: AddIdentity = dec.decode()?;
                    let k = IdentityChangeHistory::import(i.identity())?;
                    if !k.verify_all_existing_events(self.id.vault()).await? {
                        response::bad_request(&req, "invalid identity key").to_vec()?
                    } else {
                        let i = k.compute_identity_id(self.id.vault()).await?;
                        self.id.update_known_identity(&i, &k, &self.storage).await?;
                        Response::ok(req.id()).to_vec()?
                    }
                }
                _ => response::unknown_path(&req).to_vec()?,
            },
            _ => response::invalid_method(&req).to_vec()?,
        };

        Ok(res)
    }

    async fn verify(&self, data: &[u8], sig: &Signature<'_>) -> Result<bool> {
        let ours = self.id.identifier();
        let theirs = sig.identity().as_str();

        let sig = vault::Signature::new(sig.data().to_vec());

        if ours.key_id() == theirs {
            let key = self.id.get_root_public_key().await?;
            self.id.vault().verify(&sig, &key, data).await
        } else {
            let iid = IdentityIdentifier::from_key_id(theirs.to_string());
            self.id
                .verify_signature(&sig, &iid, data, &self.storage)
                .await
        }
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

    pub async fn sign(&mut self, attrs: &Attributes<'_>) -> Result<Credential<'_>> {
        let req = Request::post("/sign").body(attrs);
        self.buf = self.request("sign", "sign_request", &req).await?;
        assert_response_match("credential", &self.buf);
        let mut d = Decoder::new(&self.buf);
        let res = response("sign", &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(d.decode()?)
        } else {
            Err(error("sign", &res, &mut d))
        }
    }

    pub async fn verify(&mut self, crd: &Credential<'_>) -> Result<Attributes<'_>> {
        let req = Request::get("/verify").body(crd);
        self.buf = self.request("verify", "credential", &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response("verify", &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(d.decode()?)
        } else {
            Err(error("verify", &res, &mut d))
        }
    }

    pub async fn identity(&mut self) -> Result<GetIdentityResponse<'_>> {
        let req = Request::get("/identity");
        self.buf = self.request("get-identity", None, &req).await?;
        assert_response_match("get-identity-response", &self.buf);
        let mut d = Decoder::new(&self.buf);
        let res = response("get-identity", &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(d.decode()?)
        } else {
            Err(error("get-verify", &res, &mut d))
        }
    }

    pub async fn add_identity(&mut self, key: &[u8]) -> Result<()> {
        let req = Request::put("/identity").body(AddIdentity::new(key.into()));
        self.buf = self.request("add-identity", "add-identity", &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response("add-identity", &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error("add-identity", &res, &mut d))
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
            target: "ockam_api::signer::client",
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
        target: "ockam_api::signer::client",
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
            target: "ockam_api::signer::client",
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
