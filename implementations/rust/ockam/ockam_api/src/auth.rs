pub mod types;

use crate::{decode_option, is_ok, request};
use crate::{Error, Method, Request, Response, Status};
use core::convert::Infallible;
use core::fmt;
use minicbor::encode::Write;
use minicbor::Decoder;
use ockam_core::compat::error::Error as StdError;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{self, Address, Route, Routed, Worker};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_node::Context;
use tracing::trace;
use types::{Attribute, Attributes};

/// Auth API server.
#[derive(Debug)]
pub struct Server<S> {
    store: S,
}

#[ockam_core::worker]
impl<S: AuthenticatedStorage> Worker for Server<S> {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        let mut buf = Vec::new();
        self.on_request(msg.as_body(), &mut buf).await?;
        ctx.send(msg.return_route(), buf).await
    }
}

impl<S: AuthenticatedStorage> Server<S> {
    pub fn new(s: S) -> Self {
        Server { store: s }
    }

    async fn on_request<W>(&mut self, data: &[u8], buf: W) -> Result<(), AuthError>
    where
        W: Write<Error = Infallible>,
    {
        let mut dec = Decoder::new(data);
        let req: Request = dec.decode()?;

        trace! {
            target: "ockam_api::auth::server",
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        match req.method() {
            Some(Method::Get) => match req.path_segments::<5>().as_slice() {
                //TODO: there are other levels than control_plane. Plus,
                //      I'm unsure if this level is even useful on the rust node?
                ["v0", "control_plane", id, key] => {
                    if let Some(a) = self.store.get(id, key).await.map_err(AuthError::storage)? {
                        Response::ok(req.id())
                            .body(Attribute::new(&a))
                            .encode(buf)?
                    } else {
                        Response::not_found(req.id()).encode(buf)?
                    }
                }
                _ => {
                    let error = Error::new(req.path())
                        .with_method(Method::Post)
                        .with_message("unknown path");
                    Response::bad_request(req.id()).body(error).encode(buf)?
                }
            },
            Some(Method::Post) => match req.path_segments::<4>().as_slice() {
                ["v0", "control_plane", id] => {
                    if req.has_body() {
                        let ca: Attributes = dec.decode()?;
                        for (k, v) in ca.attrs() {
                            self.store
                                .set(id, k.to_string(), v.to_vec())
                                .await
                                .map_err(AuthError::storage)?
                        }
                        Response::ok(req.id()).encode(buf)?
                    } else {
                        let error = Error::new(req.path())
                            .with_method(Method::Post)
                            .with_message("missing request body");
                        Response::bad_request(req.id()).body(error).encode(buf)?
                    }
                }
                _ => {
                    let error = Error::new(req.path())
                        .with_method(Method::Post)
                        .with_message("unknown path");
                    Response::bad_request(req.id()).body(error).encode(buf)?
                }
            },
            Some(Method::Delete) => match req.path_segments::<5>().as_slice() {
                ["v0", "control_plane", id, key] => {
                    self.store.del(id, key).await.map_err(AuthError::storage)?;
                    Response::ok(req.id()).encode(buf)?
                }
                _ => {
                    let error = Error::new(req.path())
                        .with_method(Method::Post)
                        .with_message("unknown path");
                    Response::bad_request(req.id()).body(error).encode(buf)?
                }
            },
            Some(m) => {
                let error = Error::new(req.path()).with_method(m);
                Response::builder(req.id(), Status::MethodNotAllowed)
                    .body(error)
                    .encode(buf)?
            }
            None => {
                let error = Error::new(req.path()).with_message("unknown method");
                Response::not_implemented(req.id())
                    .body(error)
                    .encode(buf)?
            }
        }

        Ok(())
    }
}

/// Auth API client.
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
    pub async fn new(r: Route, ctx: &Context) -> ockam_core::Result<Self> {
        let ctx = ctx.new_detached(Address::random_local()).await?;
        Ok(Client {
            ctx,
            route: r,
            buf: Vec::new(),
        })
    }

    pub async fn set(&mut self, id: &str, attrs: &Attributes<'_>) -> ockam_core::Result<()> {
        let label = "set attributes";
        let req = Request::post(format!("v0/control_plane/{id}")).body(attrs);
        self.buf = request(&mut self.ctx, label, "attributes", self.route.clone(), req).await?;
        is_ok(label, &self.buf)
    }

    pub async fn get(&mut self, id: &str, attr: &str) -> ockam_core::Result<Option<&[u8]>> {
        let label = "get attribute";
        let req = Request::get(format!("v0/control_plane/{id}/{attr}"));
        self.buf = request(&mut self.ctx, label, None, self.route.clone(), req).await?;
        let a: Option<Attribute> = decode_option(label, "attribute", &self.buf)?;
        Ok(a.map(|a| a.value()))
    }

    pub async fn del(&mut self, id: &str, attr: &str) -> ockam_core::Result<()> {
        let label = "del attribute";
        let req = Request::delete(format!("/v0/control_plane/{id}/{attr}"));
        self.buf = request(&mut self.ctx, label, None, self.route.clone(), req).await?;
        is_ok(label, &self.buf)
    }
}

#[derive(Debug)]
pub struct AuthError(ErrorImpl);

impl AuthError {
    fn storage<E: StdError + Send + Sync + 'static>(e: E) -> Self {
        AuthError(ErrorImpl::Storage(Box::new(e)))
    }
}

#[derive(Debug)]
enum ErrorImpl {
    Decode(minicbor::decode::Error),
    Encode(minicbor::encode::Error<Infallible>),
    Storage(Box<dyn StdError + Send + Sync>),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ErrorImpl::Encode(e) => e.fmt(f),
            ErrorImpl::Decode(e) => e.fmt(f),
            ErrorImpl::Storage(e) => e.fmt(f),
        }
    }
}

impl ockam_core::compat::error::Error for AuthError {
    #[cfg(feature = "std")]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            ErrorImpl::Decode(e) => Some(e),
            ErrorImpl::Encode(e) => Some(e),
            ErrorImpl::Storage(e) => Some(&**e),
        }
    }
}

impl From<minicbor::decode::Error> for AuthError {
    fn from(e: minicbor::decode::Error) -> Self {
        AuthError(ErrorImpl::Decode(e))
    }
}

impl From<minicbor::encode::Error<Infallible>> for AuthError {
    fn from(e: minicbor::encode::Error<Infallible>) -> Self {
        AuthError(ErrorImpl::Encode(e))
    }
}

impl From<AuthError> for ockam_core::Error {
    fn from(e: AuthError) -> Self {
        ockam_core::Error::new(Origin::Application, Kind::Invalid, e)
    }
}

impl From<Infallible> for AuthError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
