use core::fmt;

use minicbor::Decoder;
use tracing::trace;

use ockam::identity::{AttributesEntry, IdentityAttributesReader, Identifier};
use ockam_core::api::{decode_option, Error};
use ockam_core::api::{Method, Request, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{self, Address, DenyAll, Result, Route, Routed, Worker};
use ockam_node::api::request;
use ockam_node::Context;

pub mod types;

/// Auth API server.
pub struct Server {
    store: Arc<dyn IdentityAttributesReader>,
}

#[ockam_core::worker]
impl Worker for Server {
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

impl Server {
    pub fn new(s: Arc<dyn IdentityAttributesReader>) -> Self {
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
            Some(Method::Get) => match req.path_segments::<2>().as_slice() {
                [""] => Response::ok(req.id())
                    .body(self.store.list().await?)
                    .to_vec()?,
                [id] => {
                    let identifier = Identifier::try_from(id.to_string())?;
                    if let Some(a) = self.store.get_attributes(&identifier).await? {
                        Response::ok(req.id()).body(a).to_vec()?
                    } else {
                        let err_body = Error::new(req.path())
                            .with_message(format!("identity {} not found", id));
                        Response::not_found(req.id()).body(err_body).to_vec()?
                    }
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
            .new_detached(
                Address::random_tagged("AuthClient.detached"),
                DenyAll,
                DenyAll,
            )
            .await?;
        Ok(Client {
            ctx,
            route: r,
            buf: Vec::new(),
        })
    }

    pub async fn get(&mut self, id: &str) -> ockam_core::Result<Option<AttributesEntry>> {
        let label = "get attribute";
        let req = Request::get(format!("/{id}"));
        self.buf = request(&self.ctx, self.route.clone(), req).await?;
        decode_option(label, "attribute", &self.buf)
    }
    pub async fn list(&mut self) -> ockam_core::Result<Vec<(Identifier, AttributesEntry)>> {
        let label = "list known identities";
        let req = Request::get("/");
        self.buf = request(&self.ctx, label, None, self.route.clone(), req).await?;
        let a: Option<Vec<(IdentityIdentifier, AttributesEntry)>> =
            decode_option(label, "attribute", &self.buf)?;
        Ok(a.unwrap())
    }
}
