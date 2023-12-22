use miette::IntoDiagnostic;

use minicbor::Decoder;
use tracing::trace;

use crate::nodes::BackgroundNodeClient;
use ockam::identity::{AttributesEntry, Identifier, IdentityAttributesRepository};
use ockam_core::api::{Method, RequestHeader};
use ockam_core::api::{Request, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{self, async_trait, Result, Routed, Worker};
use ockam_node::api::Client;
use ockam_node::Context;

pub mod types;

/// Auth API server.
pub struct Server {
    identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
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
    pub fn new(identity_attributes_repository: Arc<dyn IdentityAttributesRepository>) -> Self {
        Server {
            identity_attributes_repository,
        }
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
                [""] => Response::ok()
                    .with_headers(&req)
                    .body(
                        self.identity_attributes_repository
                            .list_attributes_by_identifier()
                            .await?,
                    )
                    .to_vec()?,
                [id] => {
                    let identifier = Identifier::try_from(id.to_string())?;
                    if let Some(a) = self
                        .identity_attributes_repository
                        .get_attributes(&identifier)
                        .await?
                    {
                        Response::ok().with_headers(&req).body(a).to_vec()?
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
impl AuthorizationApi for BackgroundNodeClient {
    async fn get_attributes(
        &self,
        ctx: &Context,
        identifier: &Identifier,
    ) -> miette::Result<Option<AttributesEntry>> {
        let (tcp_connection, client) = self.make_client().await?;
        let res = client.get_attributes(ctx, identifier).await;
        _ = tcp_connection.stop(ctx).await;
        res
    }

    async fn list_identifiers(
        &self,
        ctx: &Context,
    ) -> miette::Result<Vec<(Identifier, AttributesEntry)>> {
        let (tcp_connection, client) = self.make_client().await?;
        let res = client.list_identifiers(ctx).await;
        _ = tcp_connection.stop(ctx).await;
        res
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
