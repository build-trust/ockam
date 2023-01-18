use minicbor::Decoder;
use ockam::{AsyncTryClone, Context};
use ockam_core::api::{Method, Request, Response};
use ockam_core::{self, Result};

use crate::cloud::lease_manager::models::influxdb::{
    CreateTokenRequest, ListTokensRequest, RevokeTokenRequest, ShowTokenRequest,
};
use crate::cloud::CloudRequestWrapper;
use crate::nodes::NodeManagerWorker;

const TARGET: &str = "ockam_api::cloud::lease_manager";
const API_SERVICE: &str = "lease_manager_influxdb";

impl NodeManagerWorker {
    pub(crate) async fn handle_influxdb_lease_request(
        &mut self,
        ctx: &mut Context,
        dec: &mut Decoder<'_>,
        req: &Request<'_>,
        project_id: &str,
    ) -> Result<Vec<u8>> {
        let method = match req.method() {
            Some(m) => m,
            None => {
                return Ok(Response::bad_request(req.id())
                    .body("Invalid request, Method required")
                    .to_vec()?)
            }
        };
        let path = req.path();
        let segments = req.path_segments::<5>();

        match (method, segments.as_slice()) {
            (Method::Post, [.., "tokens"]) => self.create_token(ctx, dec, project_id).await,
            (Method::Get, [.., "tokens"]) => self.list_tokens(ctx, dec, project_id).await,
            (Method::Get, [.., "tokens", token_id]) => {
                self.show_token(ctx, dec, project_id, token_id).await
            }
            (Method::Delete, [.., "tokens", token_id]) => {
                self.delete_token(ctx, dec, project_id, token_id).await
            }
            _ => {
                warn!(%method, %path, "Called invalid endpoint");

                Ok(Response::bad_request(req.id())
                    .body(format!("Invalid endpoint: {}", path))
                    .to_vec()?)
            }
        }
    }

    // POST /tokens
    async fn create_token(
        &mut self,
        ctx: &mut Context,
        dec: &mut Decoder<'_>,
        project_id: &str,
    ) -> Result<Vec<u8>> {
        let req_wrapper: CloudRequestWrapper<CreateTokenRequest> = dec.decode()?;
        let cloud_route = req_wrapper.route()?;
        let req_body = req_wrapper.req;

        trace!(target: TARGET, project_id, "creating influxdb token");

        let label = "influxdb_lease_manager_create_token";

        let req_builder =
            Request::post(format!("/v0/{project_id}/lease_manager/influxdb/tokens")).body(req_body);

        let ident = {
            let inner = self.get().read().await;
            inner.identity()?.async_try_clone().await?
        };

        self.request_controller(
            ctx,
            label,
            None,
            cloud_route,
            API_SERVICE,
            req_builder,
            ident,
        )
        .await
    }

    // GET /tokens
    async fn list_tokens(
        &mut self,
        ctx: &mut Context,
        dec: &mut Decoder<'_>,
        project_id: &str,
    ) -> Result<Vec<u8>> {
        let req_wrapper: CloudRequestWrapper<ListTokensRequest> = dec.decode()?;
        let cloud_route = req_wrapper.route()?;
        let req_body = req_wrapper.req;

        trace!(target: TARGET, project_id, "listing influxdb tokens");

        let label = "influxdb_lease_manager_list_tokens";

        let req_builder =
            Request::get(format!("/v0/{project_id}/lease_manager/influxdb/tokens")).body(req_body);

        let ident = {
            let inner = self.get().read().await;
            inner.identity()?.async_try_clone().await?
        };

        self.request_controller(
            ctx,
            label,
            None,
            cloud_route,
            API_SERVICE,
            req_builder,
            ident,
        )
        .await
    }

    // GET /tokens/{id}
    async fn show_token(
        &mut self,
        ctx: &mut Context,
        dec: &mut Decoder<'_>,
        project_id: &str,
        token_id: &str,
    ) -> Result<Vec<u8>> {
        let req_wrapper: CloudRequestWrapper<ShowTokenRequest> = dec.decode()?;
        let cloud_route = req_wrapper.route()?;
        let req_body = req_wrapper.req;

        trace!(
            target: TARGET,
            project_id,
            "retrieving details for influxdb token {token_id}"
        );

        let label = "influxdb_lease_manager_show_tokens";
        let req_builder = Request::get(format!(
            "/v0/{project_id}/lease_manager/influxdb/tokens/{token_id}"
        ))
        .body(req_body);

        let ident = {
            let inner = self.get().read().await;
            inner.identity()?.async_try_clone().await?
        };

        self.request_controller(
            ctx,
            label,
            None,
            cloud_route,
            API_SERVICE,
            req_builder,
            ident,
        )
        .await
    }

    // DELETE /tokens/{id}
    async fn delete_token(
        &mut self,
        ctx: &mut Context,
        dec: &mut Decoder<'_>,
        project_id: &str,
        token_id: &str,
    ) -> Result<Vec<u8>> {
        let req_wrapper: CloudRequestWrapper<RevokeTokenRequest> = dec.decode()?;
        let cloud_route = req_wrapper.route()?;
        let req_body = req_wrapper.req;

        trace!(
            target: TARGET,
            project_id,
            "revoking influxdb token {token_id}"
        );

        let label = "influxdb_lease_manager_revoke_token";
        let req_builder = Request::delete(format!(
            "/v0/{project_id}/lease_manager/influxdb/tokens/{token_id}"
        ))
        .body(req_body);

        let ident = {
            let inner = self.get().read().await;
            inner.identity()?.async_try_clone().await?
        };

        self.request_controller(
            ctx,
            label,
            None,
            cloud_route,
            API_SERVICE,
            req_builder,
            ident,
        )
        .await
    }
}
