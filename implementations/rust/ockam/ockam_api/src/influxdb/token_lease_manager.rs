use crate::cloud::lease_manager::models::influxdb::Token;
use crate::nodes::service::encode_response;
use minicbor::{Decoder, Encode};
use ockam_core::api::Method::{Delete, Get, Post};
use ockam_core::api::{RequestHeader, Response};
use ockam_core::{async_trait, Address, Routed, Worker};
use ockam_node::Context;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct InfluxDbTokenLeaseManagerWorker {
    inner: Arc<Mutex<InfluxDbTokenLeaseManagerInner>>,
}

impl InfluxDbTokenLeaseManagerWorker {
    pub(crate) fn new(
        address: Address,
        influxdb_org_id: String,
        influxdb_token: String,
        token_permissions: String,
        token_ttl: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InfluxDbTokenLeaseManagerInner {
                address,
                influxdb_org_id,
                influxdb_token,
                token_permissions,
                token_ttl,
            })),
        }
    }

    #[instrument(skip_all, fields(method = ?req.method(), path = req.path()))]
    async fn handle_request(
        &mut self,
        ctx: &mut Context,
        req: &RequestHeader,
        _dec: &mut Decoder<'_>,
    ) -> ockam_core::Result<Vec<u8>> {
        debug! {
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        let path = req.path();
        let path_segments = req.path_segments::<5>();
        let method = match req.method() {
            Some(m) => m,
            None => todo!(),
        };

        let r = match (method, path_segments.as_slice()) {
            (Post, []) => encode_response(req, self.create_token(ctx).await)?,
            (Get, [token_id]) => encode_response(req, self.get_token(ctx, token_id).await)?,
            (Delete, [token_id]) => encode_response(req, self.revoke_token(ctx, token_id).await)?,
            (Get, []) => encode_response(req, self.list_tokens(ctx).await)?,
            // ==*== Catch-all for Unimplemented APIs ==*==
            _ => {
                warn!(%method, %path, "Called invalid endpoint");
                Response::bad_request(req, &format!("Invalid endpoint: {} {}", method, path))
                    .to_vec()?
            }
        };
        Ok(r)
    }
}

#[ockam::worker]
impl Worker for InfluxDbTokenLeaseManagerWorker {
    type Message = Vec<u8>;
    type Context = Context;

    async fn shutdown(&mut self, _ctx: &mut Self::Context) -> ockam_core::Result<()> {
        debug!("Shutting down InfluxDbTokenLeaseManagerWorker");
        Ok(())
    }

    #[instrument(skip_all, name = "InfluxDbTokenLeaseWorker::handle_message")]
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Vec<u8>>,
    ) -> ockam_core::Result<()> {
        let return_route = msg.return_route();
        let body = msg.into_body()?;
        let mut dec = Decoder::new(&body);
        let req: RequestHeader = match dec.decode() {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to decode request: {:?}", e);
                return Ok(());
            }
        };

        let r = match self.handle_request(ctx, &req, &mut dec).await {
            Ok(r) => r,
            Err(err) => {
                error! {
                    re     = %req.id(),
                    method = ?req.method(),
                    path   = %req.path(),
                    code   = %err.code(),
                    cause  = ?err.source(),
                    "failed to handle request"
                }
                Response::internal_error(&req, &format!("failed to handle request: {err} {req:?}"))
                    .to_vec()?
            }
        };
        debug! {
            re     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            "responding"
        }
        ctx.send(return_route, r).await
    }
}

pub(crate) struct InfluxDbTokenLeaseManagerInner {
    address: Address,
    influxdb_org_id: String,
    influxdb_token: String,
    token_permissions: String,
    token_ttl: Duration,
}

#[async_trait]
pub trait InfluxDbTokenLeaseManagerTrait {
    async fn create_token(
        &self,
        ctx: &Context,
    ) -> Result<Response<Token>, Response<ockam_core::api::Error>>;

    async fn get_token(
        &self,
        ctx: &Context,
        token_id: &str,
    ) -> Result<Response<Token>, Response<ockam_core::api::Error>>;

    async fn revoke_token(
        &self,
        ctx: &Context,
        token_id: &str,
    ) -> Result<Response, Response<ockam_core::api::Error>>;

    async fn list_tokens(
        &self,
        ctx: &Context,
    ) -> Result<Response<Vec<Token>>, Response<ockam_core::api::Error>>;
}

#[async_trait]
impl InfluxDbTokenLeaseManagerTrait for InfluxDbTokenLeaseManagerWorker {
    async fn create_token(
        &self,
        ctx: &Context,
    ) -> Result<Response<Token>, Response<ockam_core::api::Error>> {
        todo!()
    }

    async fn get_token(
        &self,
        ctx: &Context,
        token_id: &str,
    ) -> Result<Response<Token>, Response<ockam_core::api::Error>> {
        todo!()
    }

    async fn revoke_token(
        &self,
        ctx: &Context,
        token_id: &str,
    ) -> Result<Response, Response<ockam_core::api::Error>> {
        todo!()
    }

    async fn list_tokens(
        &self,
        ctx: &Context,
    ) -> Result<Response<Vec<Token>>, Response<ockam_core::api::Error>> {
        todo!()
    }
}
