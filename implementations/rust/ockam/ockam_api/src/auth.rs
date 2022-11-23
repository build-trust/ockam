pub mod types;

use core::fmt;
use minicbor::Decoder;
use ockam_core::api::{decode_option, is_ok};
use ockam_core::api::{Method, Request, Response};
use ockam_core::{self, Address, DenyAll, Result, Route, Routed, Worker};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_node::api::request;
use ockam_node::Context;
use std::sync::Arc;
use tracing::trace;
use types::Attribute;

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
        let r = self.on_request(msg.as_body()).await?;
        ctx.send(msg.return_route(), r).await
    }
}

impl<S: AuthenticatedStorage> Server<S> {
    pub fn new(s: S) -> Self {
        Server { store: s }
    }

    async fn on_request(&mut self, data: &[u8]) -> Result<Vec<u8>> {
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

        let res = match req.method() {
            Some(Method::Get) => match req.path_segments::<5>().as_slice() {
                ["authenticated", id, "attribute", key] => {
                    if let Some(a) = self.store.get(id, key).await? {
                        Response::ok(req.id()).body(Attribute::new(&a)).to_vec()?
                    } else {
                        Response::not_found(req.id()).to_vec()?
                    }
                }
                _ => ockam_core::api::unknown_path(&req).to_vec()?,
            },
            Some(Method::Delete) => match req.path_segments::<5>().as_slice() {
                ["authenticated", id, "attribute", key] => {
                    self.store.del(id, key).await?;
                    Response::ok(req.id()).to_vec()?
                }
                _ => ockam_core::api::unknown_path(&req).to_vec()?,
            },
            _ => ockam_core::api::invalid_method(&req).to_vec()?,
        };

        Ok(res)
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
        let ctx = ctx
            .new_detached_with_access_control(
                Address::random_tagged("AuthClient.detached"),
                Arc::new(DenyAll),
                Arc::new(DenyAll),
            )
            .await?;
        Ok(Client {
            ctx,
            route: r,
            buf: Vec::new(),
        })
    }

    pub async fn get(&mut self, id: &str, attr: &str) -> ockam_core::Result<Option<&[u8]>> {
        let label = "get attribute";
        let req = Request::get(format!("/authenticated/{id}/attribute/{attr}"));
        self.buf = request(&mut self.ctx, label, None, self.route.clone(), req).await?;
        let a: Option<Attribute> = decode_option(label, "attribute", &self.buf)?;
        Ok(a.map(|a| a.value()))
    }

    pub async fn del(&mut self, id: &str, attr: &str) -> ockam_core::Result<()> {
        let label = "del attribute";
        let req = Request::delete(format!("/authenticated/{id}/attribute/{attr}"));
        self.buf = request(&mut self.ctx, label, None, self.route.clone(), req).await?;
        is_ok(label, &self.buf)
    }
}
