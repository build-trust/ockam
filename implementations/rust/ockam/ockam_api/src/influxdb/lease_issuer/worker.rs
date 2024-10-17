use crate::influxdb::influxdb_api_client::{
    InfluxDBApi, InfluxDBApiClient, InfluxDBCreateTokenRequest,
};
use crate::influxdb::lease_issuer::node_service::InfluxDBTokenLessorState;
use crate::influxdb::lease_token::LeaseToken;
use crate::nodes::service::encode_response;
use crate::ApiError;
use minicbor::Decoder;
use ockam::identity::Identifier;
use ockam_core::api::Method::{Delete, Get, Post};
use ockam_core::api::{RequestHeader, Response};
use ockam_core::{async_trait, Address, Routed, SecureChannelLocalInfo, Worker};
use ockam_node::Context;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::RwLock;

#[derive(Clone)]
pub(crate) struct InfluxDBTokenLessorWorker {
    pub(crate) state: Arc<RwLock<InfluxDBTokenLessorState>>,
}

impl InfluxDBTokenLessorWorker {
    pub(crate) async fn new(
        address: Address,
        influxdb_address: String,
        influxdb_org_id: String,
        influxdb_token: String,
        token_permissions: String,
        token_ttl: Duration,
    ) -> ockam_core::Result<Self> {
        debug!("Creating InfluxDBTokenLessorWorker");
        let _self = Self {
            state: Arc::new(RwLock::new(InfluxDBTokenLessorState {
                address,
                influxdb_api_client: InfluxDBApiClient::new(influxdb_address, influxdb_token)?,
                influxdb_org_id,
                token_permissions,
                token_ttl,
                active_tokens: BinaryHeap::new(),
            })),
        };
        Ok(_self)
    }

    #[instrument(skip_all, fields(method = ?req.method(), path = req.path()))]
    async fn handle_request(
        &mut self,
        _ctx: &mut Context,
        requester: &Identifier,
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
        debug!(path_segments = ?path_segments.as_slice().iter().map(|s| s.to_string()).collect::<Vec<_>>(), "Handling request");

        // [""] correspond to the root "/" path
        let r = match (method, path_segments.as_slice()) {
            (Post, [""]) => encode_response(req, self.create_token(requester).await)?,
            (Get, [""]) => encode_response(req, self.list_tokens(requester).await)?,
            (Get, [token_id]) => encode_response(req, self.get_token(requester, token_id).await)?,
            (Delete, [token_id]) => {
                encode_response(req, self.revoke_token(requester, token_id).await)?
            }
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
impl Worker for InfluxDBTokenLessorWorker {
    type Message = Vec<u8>;
    type Context = Context;

    async fn shutdown(&mut self, _ctx: &mut Self::Context) -> ockam_core::Result<()> {
        debug!("Shutting down InfluxDBTokenLessorWorker");
        Ok(())
    }

    #[instrument(skip_all, name = "InfluxDBTokenLessorWorker::handle_message")]
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Vec<u8>>,
    ) -> ockam_core::Result<()> {
        let requester_identifier = Identifier::from(
            SecureChannelLocalInfo::find_info(msg.local_message())?.their_identifier(),
        );

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

        let r = match self
            .handle_request(ctx, &requester_identifier, &req, &mut dec)
            .await
        {
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

#[async_trait]
pub trait InfluxDBTokenLessorWorkerApi {
    async fn create_token(
        &self,
        requester: &Identifier,
    ) -> Result<Response<LeaseToken>, Response<ockam_core::api::Error>>;

    async fn get_token(
        &self,
        requester: &Identifier,
        token_id: &str,
    ) -> Result<Response<LeaseToken>, Response<ockam_core::api::Error>>;

    async fn revoke_token(
        &self,
        requester: &Identifier,
        token_id: &str,
    ) -> Result<Response, Response<ockam_core::api::Error>>;

    async fn list_tokens(
        &self,
        requester: &Identifier,
    ) -> Result<Response<Vec<LeaseToken>>, Response<ockam_core::api::Error>>;
}

#[async_trait]
impl InfluxDBTokenLessorWorkerApi for InfluxDBTokenLessorWorker {
    async fn create_token(
        &self,
        requester: &Identifier,
    ) -> Result<Response<LeaseToken>, Response<ockam_core::api::Error>> {
        debug!(%requester, "Creating token");
        let influxdb_token = {
            let state = self.state.read().await;
            let expires = OffsetDateTime::now_utc() + state.token_ttl;
            state
                .influxdb_api_client
                .create_token(InfluxDBCreateTokenRequest::new(
                    state.influxdb_org_id.clone(),
                    state.token_permissions.clone(),
                    requester,
                    expires,
                ))
                .await?
                .into_response()?
        };
        let lease_token: Option<LeaseToken> = influxdb_token.try_into()?;
        match lease_token {
            Some(lease_token) => {
                {
                    let mut state = self.state.write().await;
                    state.active_tokens.push(Reverse(lease_token.clone()));
                }
                Ok(Response::ok().body(lease_token))
            }
            None => {
                warn!("Token does not contain Ockam metadata, ignoring");
                Err(Response::bad_request_no_request(
                    "Token does not contain Ockam metadata",
                ))
            }
        }
    }

    async fn get_token(
        &self,
        requester: &Identifier,
        token_id: &str,
    ) -> Result<Response<LeaseToken>, Response<ockam_core::api::Error>> {
        debug!(%requester, %token_id, "Getting token");
        let influxdb_token = {
            let state = self.state.read().await;
            state
                .influxdb_api_client
                .get_token(token_id)
                .await?
                .into_response()?
        };
        debug!(%requester, %token_id, "Received token: {:?}", influxdb_token);
        let lease_token: Option<LeaseToken> = influxdb_token.try_into().map_err(|e| {
            ApiError::core(format!(
                "Failed to parse InfluxDB token as a LeaseToken: {e}"
            ))
        })?;
        match lease_token {
            Some(lease_token) => {
                if requester.eq(&lease_token.issued_for) {
                    Ok(Response::ok().body(lease_token))
                } else {
                    warn!(%requester, %token_id, "Token not authorized");
                    Err(Response::unauthorized_no_request(
                        "Token not authorized for requester",
                    ))
                }
            }
            None => {
                warn!(%requester, %token_id, "Token not found");
                Err(Response::not_found_no_request(
                    "Token does not contain Ockam metadata",
                ))
            }
        }
    }

    async fn revoke_token(
        &self,
        requester: &Identifier,
        token_id: &str,
    ) -> Result<Response, Response<ockam_core::api::Error>> {
        debug!(%requester, %token_id, "Revoking token");
        let is_authorized_to_revoke = self
            .get_token(requester, token_id)
            .await?
            .into_parts()
            .1
            .is_some();
        if !is_authorized_to_revoke {
            return Err(Response::unauthorized_no_request(
                "Not authorized to revoke token",
            ));
        }
        let revoked = {
            let state = self.state.read().await;
            state
                .influxdb_api_client
                .revoke_token(token_id)
                .await?
                .into_response()?;
            true
        };
        if revoked {
            info!(%requester, %token_id, "Token revoked");
            {
                let mut state = self.state.write().await;
                state.active_tokens.retain(|t| t.0.id != token_id);
            }
            Ok(Response::ok())
        } else {
            Err(Response::internal_error_no_request(
                "Failed to revoke token",
            ))
        }
    }

    async fn list_tokens(
        &self,
        requester: &Identifier,
    ) -> Result<Response<Vec<LeaseToken>>, Response<ockam_core::api::Error>> {
        debug!(%requester, "Listing tokens");
        let influxdb_tokens = {
            let state = self.state.read().await;
            state
                .influxdb_api_client
                .list_tokens()
                .await?
                .into_response()?
                .tokens
        };
        debug!("Received tokens list: {:?}", influxdb_tokens);
        let lease_tokens: Vec<LeaseToken> = influxdb_tokens
            .into_iter()
            .filter_map(|token| {
                let lease_token: Result<Option<LeaseToken>, _> = token.try_into();
                if let Some(lease_token) = lease_token.ok().flatten() {
                    if requester.eq(&lease_token.issued_for) {
                        Some(lease_token)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        {
            let mut state = self.state.write().await;
            state.active_tokens = lease_tokens.iter().map(|t| Reverse(t.clone())).collect();
        }
        info!("Found {} tokens", lease_tokens.len());
        Ok(Response::ok().body(lease_tokens))
    }
}
