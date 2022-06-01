pub mod store;
pub mod types;

use crate::{Error, Method, Request, RequestBuilder, Response, Status};
use core::convert::Infallible;
use core::fmt;
use minicbor::encode::Write;
use minicbor::{Decoder, Encode};
use ockam_core::compat::error::Error as StdError;
use ockam_core::compat::io;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{self, Address, Route, Routed, Worker};
use ockam_node::Context;
use store::Storage;
use tracing::{trace, warn};
use types::{Attribute, Attributes};

/// Auth API server.
#[derive(Debug)]
pub struct Server<S> {
    store: S,
}

#[ockam_core::worker]
impl<S: Storage + Send + Sync + 'static> Worker for Server<S> {
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

impl<S: Storage + Send + Sync + 'static> Server<S> {
    pub fn new(s: S) -> Self {
        Server { store: s }
    }

    async fn on_request<W>(&mut self, data: &[u8], buf: W) -> Result<(), AuthError>
    where
        W: Write<Error = io::Error>,
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
                ["subject", id, "attribute", key] => {
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
            Some(Method::Post) => match req.path_segments::<3>().as_slice() {
                ["subject", id] => {
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
                ["subject", id, "attribute", key] => {
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

    pub async fn set(&mut self, subj: &str, attrs: &Attributes<'_>) -> ockam_core::Result<()> {
        let req = Request::post(format!("/subject/{subj}")).body(attrs);
        self.buf = self.request("set attributes", &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response("set attributes", &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error("set attributes", &res, &mut d))
        }
    }

    pub async fn get(&mut self, subj: &str, attr: &str) -> ockam_core::Result<Option<&[u8]>> {
        let req = Request::get(format!("/subject/{subj}/attribute/{attr}"));
        self.buf = self.request("get attribute", &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response("get attribute", &mut d)?;
        match res.status() {
            Some(Status::Ok) => {
                let a: Attribute = d.decode()?;
                Ok(Some(a.value()))
            }
            Some(Status::NotFound) => Ok(None),
            _ => Err(error("get attribute", &res, &mut d)),
        }
    }

    pub async fn del(&mut self, subj: &str, attr: &str) -> ockam_core::Result<()> {
        let req = Request::delete(format!("/subject/{subj}/attribute/{attr}"));
        self.buf = self.request("del attribute", &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response("del attribute", &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error("del attribute", &res, &mut d))
        }
    }

    /// Encode request header and body (if any) and send the package to the server.
    async fn request<T>(
        &mut self,
        label: &str,
        req: &RequestBuilder<'_, T>,
    ) -> ockam_core::Result<Vec<u8>>
    where
        T: Encode<()>,
    {
        let mut buf = Vec::new();
        req.encode(&mut buf)?;
        trace! {
            target: "ockam_api::auth::client",
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
fn response(label: &str, dec: &mut Decoder<'_>) -> ockam_core::Result<Response> {
    let res: Response = dec.decode()?;
    trace! {
        target: "ockam_api::auth::client",
        re     = %res.re(),
        id     = %res.id(),
        status = ?res.status(),
        body   = %res.has_body(),
        "<- {label}"
    }
    Ok(res)
}

/// Decode, log and mape response error to ockam_core error.
fn error(label: &str, res: &Response, dec: &mut Decoder<'_>) -> ockam_core::Error {
    if res.has_body() {
        let err = match dec.decode::<Error>() {
            Ok(e) => e,
            Err(e) => return e.into(),
        };
        warn! {
            target: "ockam_api::auth::client",
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
    Encode(minicbor::encode::Error<io::Error>),
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

impl From<minicbor::encode::Error<io::Error>> for AuthError {
    fn from(e: minicbor::encode::Error<io::Error>) -> Self {
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
