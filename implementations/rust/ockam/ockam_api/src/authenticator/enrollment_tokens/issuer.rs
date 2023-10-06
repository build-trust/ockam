use miette::IntoDiagnostic;
use minicbor::Decoder;
use ockam::identity::AttributesEntry;
use ockam::identity::{Identifier, IdentitySecureChannelLocalInfo};
use ockam_core::api::{Method, Request, RequestHeader, Response};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Result, Routed, Worker};
use ockam_node::Context;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::trace;

use crate::authenticator::direct::types::{AddMember, CreateToken};
use crate::authenticator::enrollment_tokens::authenticator::MAX_TOKEN_DURATION;
use crate::authenticator::enrollment_tokens::types::Token;
use crate::authenticator::enrollment_tokens::EnrollmentTokenAuthenticator;
use crate::authenticator::one_time_code::OneTimeCode;
use crate::authenticator::secure_channel_required;
use crate::cloud::AuthorityNode;
use crate::DefaultAddress;

pub struct EnrollmentTokenIssuer {
    pub(super) authenticator: EnrollmentTokenAuthenticator,
}

impl EnrollmentTokenIssuer {
    async fn issue_token(
        &self,
        enroller: &Identifier,
        attrs: HashMap<String, String>,
        token_duration: Option<Duration>,
        ttl_count: Option<u64>,
    ) -> Result<OneTimeCode> {
        let otc = OneTimeCode::new();
        let max_token_duration = token_duration.unwrap_or(MAX_TOKEN_DURATION);
        let ttl_count = ttl_count.unwrap_or(1);
        let tkn = Token {
            attrs,
            issued_by: enroller.clone(),
            created_at: Instant::now(),
            ttl: max_token_duration,
            ttl_count,
        };
        self.authenticator
            .tokens
            .write()
            .map(|mut r| {
                r.insert(*otc.code(), tkn);
                otc
            })
            .map_err(|_| {
                ockam_core::Error::new(
                    Origin::Other,
                    Kind::Internal,
                    "failed to get read lock on tokens table",
                )
            })
    }
    pub fn new(authenticator: EnrollmentTokenAuthenticator) -> Self {
        Self { authenticator }
    }
}

#[ockam_core::worker]
impl Worker for EnrollmentTokenIssuer {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let from = i.their_identity_id();
            let mut dec = Decoder::new(m.as_body());
            let req: RequestHeader = dec.decode()?;
            trace! {
                target: "ockam_api::authenticator::direct::enrollment_token_issuer",
                from   = %from,
                id     = %req.id(),
                method = ?req.method(),
                path   = %req.path(),
                body   = %req.has_body(),
                "request"
            }
            let res = match (req.method(), req.path()) {
                (Some(Method::Post), "/") | (Some(Method::Post), "/tokens") => {
                    let att: CreateToken = dec.decode()?;
                    let duration = att.ttl_secs().map(Duration::from_secs);
                    let ttl_count = att.ttl_count();
                    // TODO: Use ttl_duration
                    match self
                        .issue_token(&from, att.into_owned_attributes(), duration, ttl_count)
                        .await
                    {
                        Ok(otc) => Response::ok(&req).body(&otc).to_vec()?,
                        Err(error) => {
                            Response::internal_error(&req, &error.to_string()).to_vec()?
                        }
                    }
                }
                _ => Response::unknown_path(&req).to_vec()?,
            };
            c.send(m.return_route(), res).await
        } else {
            secure_channel_required(c, m).await
        }
    }
}

#[async_trait]
pub trait Members {
    async fn add_member(
        &self,
        ctx: &Context,
        identifier: Identifier,
        attributes: HashMap<&str, &str>,
    ) -> miette::Result<()>;

    async fn delete_member(&self, ctx: &Context, identifier: Identifier) -> miette::Result<()>;

    async fn list_member_ids(&self, ctx: &Context) -> miette::Result<Vec<Identifier>>;

    async fn list_members(
        &self,
        ctx: &Context,
    ) -> miette::Result<HashMap<Identifier, AttributesEntry>>;
}

#[async_trait]
impl Members for AuthorityNode {
    async fn add_member(
        &self,
        ctx: &Context,
        identifier: Identifier,
        attributes: HashMap<&str, &str>,
    ) -> miette::Result<()> {
        let req = Request::post("/").body(AddMember::new(identifier).with_attributes(attributes));
        self.0
            .tell(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn delete_member(&self, ctx: &Context, identifier: Identifier) -> miette::Result<()> {
        let req = Request::delete(format!("/{identifier}"));
        self.0
            .tell(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn list_member_ids(&self, ctx: &Context) -> miette::Result<Vec<Identifier>> {
        let req = Request::get("/member_ids");
        self.0
            .ask(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn list_members(
        &self,
        ctx: &Context,
    ) -> miette::Result<HashMap<Identifier, AttributesEntry>> {
        let req = Request::get("/");
        self.0
            .ask(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}

#[async_trait]
pub trait TokenIssuer {
    async fn create_token(
        &self,
        ctx: &Context,
        attributes: HashMap<&str, &str>,
        duration: Option<Duration>,
        ttl_count: Option<u64>,
    ) -> miette::Result<OneTimeCode>;
}

#[async_trait]
impl TokenIssuer for AuthorityNode {
    async fn create_token(
        &self,
        ctx: &Context,
        attributes: HashMap<&str, &str>,
        duration: Option<Duration>,
        ttl_count: Option<u64>,
    ) -> miette::Result<OneTimeCode> {
        let body = CreateToken::new()
            .with_attributes(attributes)
            .with_ttl(duration)
            .with_ttl_count(ttl_count);

        let req = Request::post("/").body(body);
        self.0
            .ask(ctx, DefaultAddress::ENROLLMENT_TOKEN_ISSUER, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}

#[async_trait]
pub trait TokenAcceptor {
    async fn present_token(&self, ctx: &Context, token: OneTimeCode) -> miette::Result<()>;
}

#[async_trait]
impl TokenAcceptor for AuthorityNode {
    async fn present_token(&self, ctx: &Context, token: OneTimeCode) -> miette::Result<()> {
        let req = Request::post("/").body(token);
        self.0
            .ask(ctx, DefaultAddress::DIRECT_AUTHENTICATOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
