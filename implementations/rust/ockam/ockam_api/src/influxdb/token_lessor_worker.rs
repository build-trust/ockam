use crate::cloud::lease_manager::models::influxdb::Token;
use crate::nodes::service::encode_response;
use crate::ApiError;
use chrono::{DateTime, NaiveDateTime};
use minicbor::Decoder;
use ockam::compat::time::now;
use ockam::identity::{Identifier, IdentitySecureChannelLocalInfo};
use ockam_core::api::Method::{Delete, Get, Post};
use ockam_core::api::{RequestHeader, Response};
use ockam_core::{async_trait, Address, Routed, Worker};
use ockam_node::Context;
use reqwest::Client;
use tokio::sync::Mutex;
use std::borrow::Cow;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tracing_core::field::debug;

#[derive(Clone)]
pub(crate) struct InfluxDbTokenLessorWorker {
    inner: Arc<Mutex<InfluxDbTokenLessorInner>>,
}

impl InfluxDbTokenLessorWorker {
    pub(crate) fn new(
        address: Address,
        influxdb_address: String,
        influxdb_org_id: String,
        influxdb_token: String,
        token_permissions: String,
        token_ttl: i32,
    ) -> Self {
        debug!("Creating InfluxDbTokenLeaseManagerWorker");


        //TODO: list all (OCKAM-generated) tokens on influxdb,  and revoke from influxdb the ones
        //      that must be expired.  This because we might have generated tokens, and then the 
        //      node got restarted,  need to check which tokens must expire.

        let http_client = reqwest::ClientBuilder::new()
            .build()
            .unwrap();

        Self {
            inner: Arc::new(Mutex::new(InfluxDbTokenLessorInner {
                address,
                influxdb_address,
                influxdb_org_id,
                influxdb_token,
                token_permissions,
                token_ttl,
                http_client
            })),
        }
    }

    #[instrument(skip_all, fields(method = ?req.method(), path = req.path()))]
    async fn handle_request(
        &mut self,
        ctx: &mut Context,
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

        let r = match (method, path_segments.as_slice()) {
            (Post, [""]) => encode_response(req, self.create_token(ctx, requester).await)?,
            (Get, [""]) => encode_response(req, self.list_tokens(ctx, requester).await)?,
            (Get, [token_id]) => encode_response(req, self.get_token(ctx, requester, token_id).await)?,
            (Delete, [token_id]) => encode_response(req, self.revoke_token(ctx, requester, token_id).await)?,
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
impl Worker for InfluxDbTokenLessorWorker {
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
        let requester_identifier = IdentitySecureChannelLocalInfo::find_info(msg.local_message())?.their_identity_id();

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

        let r = match self.handle_request(ctx, &requester_identifier, &req, &mut dec).await {
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

pub(crate) struct InfluxDbTokenLessorInner {
    address: Address,
    influxdb_address: String,
    influxdb_org_id: String,
    influxdb_token: String,
    token_permissions: String,
    token_ttl: i32,
    http_client: Client,
}

#[async_trait]
pub trait InfluxDbTokenLessorWorkerTrait {
    async fn create_token(
        &self,
        ctx: &Context,
        requester: &Identifier,
    ) -> Result<Response<Token>, Response<ockam_core::api::Error>>;

    async fn get_token(
        &self,
        ctx: &Context,
        requester: &Identifier,
        token_id: &str,
    ) -> Result<Response<Token>, Response<ockam_core::api::Error>>;

    async fn revoke_token(
        &self,
        ctx: &Context,
        requester: &Identifier,
        token_id: &str,
    ) -> Result<Response, Response<ockam_core::api::Error>>;

    async fn list_tokens(
        &self,
        ctx: &Context,
        requester: &Identifier,
    ) -> Result<Response<Vec<Token>>, Response<ockam_core::api::Error>>;
}


// To parse the json returned by influxdb API
#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
pub struct InfluxDBToken {
    pub id: String,
    pub description: String,
    pub token: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}


//TODO fixme:  *Not* all tokens will be generated by us.  Here should detect if it's a token
//              managed by us (if the unpack_metadata found a valid ockam metadata there), and if
//              not, act as if this token didn't exist (so skip it when listing, or when retrieving by id)
impl From<InfluxDBToken> for Token {
    fn from(value: InfluxDBToken) -> Self {
        let (issued_for, expires) = unpack_metadata(&value.description).unwrap();
        let expires = DateTime::from_timestamp(expires as i64, 0).unwrap();

        Self{
            id:  value.id,
            issued_for: issued_for.to_string(),
            created_at: value.created_at.clone(),
            expires: expires.to_rfc3339(), //format!("{}", expires), 
            status: value.status,
            token: value.token
        }
    }
}


// There are no "tags" on the tokens on influxdb where to store metadata, just a description field.
// We need to pack 2 things there:  the identifer for which we created this token, and when
// the token must expire.
fn pack_metadata(identifier: &Identifier, expires: u64) -> String {
    format!("OCKAM:{}:{}", identifier.to_string(), expires).to_string()
}

fn unpack_metadata(description: &str) -> Option<(Identifier, u64)> {
    let v : Vec<&str>  = description.split(":").collect();
    match v[..] {
        ["OCKAM", identifier, expires] =>  {
            let identifier = Identifier::try_from(identifier).unwrap();
            let expires : u64 = expires.parse().unwrap();
            Some((identifier, expires))
        },
        _ => None
    }
}

#[async_trait]
impl InfluxDbTokenLessorWorkerTrait for InfluxDbTokenLessorWorker {
    async fn create_token(
        &self,
        ctx: &Context,
        requester: &Identifier,
    ) -> Result<Response<Token>, Response<ockam_core::api::Error>> {
        debug!("Creating token");
        let state = self.inner.lock().await;
        let expires = now().unwrap() + state.token_ttl as u64;
        let description = pack_metadata(requester, expires);

        let req = state.http_client.post(format!("{}/api/v2/authorizations", state.influxdb_address))
                .header("Authorization", format!("Token {}", state.influxdb_token))
                .header("Content-Type", "application/json")
                .body(format!("{{\"description\": \"{}\", \"orgID\": \"{}\", \"permissions\":{}}}", description, state.influxdb_org_id, state.token_permissions));

        //TODO FIXME: this can fail, be rejected, etc.
        let res = req.send().await.unwrap();
        let token = res.json::<InfluxDBToken>().await.unwrap();
        let token = Token::from(token);
        info!("Generated token!!: {:?}", token);

        //TODO: schedule a deletion of this token at the TTL/expiration date.

        Ok(Response::ok().body(token))
    }

    async fn get_token(
        &self,
        ctx: &Context,
        requester: &Identifier,
        token_id: &str,
    ) -> Result<Response<Token>, Response<ockam_core::api::Error>> {
        // TODO https://docs.influxdata.com/influxdb/v2/api/#operation/GetAuthorizationsID
        //  NOTE!!!: check if the identifier that created it is the same
        //           one requesting it.  Otherwise the user is not authorized for doing it.
        debug!("Getting token");
        Ok(Response::ok().body(Token {
            id: "token_id".to_string(),
            issued_for: "".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            expires: "2024-01-01T00:00:00Z".to_string(),
            token: "asdfg".to_string(),
            status: "".to_string(),
        }))
    }

    async fn revoke_token(
        &self,
        ctx: &Context,
        requester: &Identifier,
        token_id: &str,
    ) -> Result<Response, Response<ockam_core::api::Error>> {
        // TODO: https://docs.influxdata.com/influxdb/v2/api/#operation/DeleteAuthorizationsID
        //  NOTE!!!: first retrieve the token,  check if the identifier that created it is the same
        //           one deleting it,  and if so revoke it.  Otherwise the user is not authorized for doing it.
        debug!("Revoking token");
        Ok(Response::ok())
    }

    async fn list_tokens(
        &self,
        ctx: &Context,
        requester: &Identifier,
    ) -> Result<Response<Vec<Token>>, Response<ockam_core::api::Error>> {
        // TODO:  https://docs.influxdata.com/influxdb/v2/api/#operation/GetAuthorizations
        // list all tokens.  Filter those that are created for this specific requester,
        // return those.
        // Yes, it's going to be very inneficient,  but it's an operation that almost never is going
        // to be used.  We can work latter to keep a local cache.
        debug!("Listing tokens");
        Ok(Response::ok().body(vec![Token {
            id: "token_id".to_string(),
            issued_for: "".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            expires: "2024-01-01T00:00:00Z".to_string(),
            token: "asdfg".to_string(),
            status: "".to_string(),
        }]))
    }
}
