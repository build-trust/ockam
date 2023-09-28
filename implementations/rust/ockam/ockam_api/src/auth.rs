use miette::IntoDiagnostic;

use minicbor::Decoder;
use tracing::trace;

use crate::nodes::BackgroundNode;
use ockam::identity::{AttributesEntry, Identifier, IdentityAttributesReader};
use ockam_core::api::{Method, RequestHeader};
use ockam_core::api::{Request, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{self, async_trait, Result, Routed, Worker};
use ockam_node::api::Client;
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
    ) -> Result<()> {
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
        let req: RequestHeader = dec.decode()?;

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
                [""] => Response::ok(&req).body(self.store.list().await?).to_vec()?,
                [id] => {
                    let identifier = Identifier::try_from(id.to_string())?;
                    if let Some(a) = self.store.get_attributes(&identifier).await? {
                        Response::ok(&req).body(a).to_vec()?
                    } else {
                        Response::not_found(&req, &format!("identity {} not found", id)).to_vec()?
                    }
                }
                _ => Response::unknown_path(&req).to_vec()?,
            },
            _ => Response::invalid_method(&req).to_vec()?,
        };

        Ok(res)
    }
}

#[async_trait]
pub trait AuthorizationApi {
    async fn get_attributes(
        &self,
        ctx: &Context,
        identifier: &Identifier,
    ) -> miette::Result<Option<AttributesEntry>>;

    async fn list_identifiers(
        &self,
        ctx: &Context,
    ) -> miette::Result<Vec<(Identifier, AttributesEntry)>>;
}

#[async_trait]
impl AuthorizationApi for BackgroundNode {
    async fn get_attributes(
        &self,
        ctx: &Context,
        identifier: &Identifier,
    ) -> miette::Result<Option<AttributesEntry>> {
        self.make_client()
            .await?
            .get_attributes(ctx, identifier)
            .await
    }

    async fn list_identifiers(
        &self,
        ctx: &Context,
    ) -> miette::Result<Vec<(Identifier, AttributesEntry)>> {
        self.make_client().await?.list_identifiers(ctx).await
    }
}

#[async_trait]
impl AuthorizationApi for Client {
    async fn get_attributes(
        &self,
        ctx: &Context,
        identifier: &Identifier,
    ) -> miette::Result<Option<AttributesEntry>> {
        let req = Request::get(format!("/{identifier}"));
        self.ask(ctx, req)
            .await
            .into_diagnostic()?
            .found()
            .into_diagnostic()
    }

    async fn list_identifiers(
        &self,
        ctx: &Context,
    ) -> miette::Result<Vec<(Identifier, AttributesEntry)>> {
        self.ask::<(), Vec<(Identifier, AttributesEntry)>>(ctx, Request::get("/"))
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
