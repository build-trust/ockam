use miette::IntoDiagnostic;
use std::collections::BTreeMap;

use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_core::compat::time::Duration;
use ockam_node::Context;

use crate::authenticator::direct::types::CreateToken;
use crate::authenticator::one_time_code::OneTimeCode;
use crate::nodes::service::default_address::DefaultAddress;
use crate::orchestrator::{AuthorityNodeClient, HasSecureClient};

#[async_trait]
pub trait TokenIssuer {
    async fn create_token(
        &self,
        ctx: &Context,
        attributes: BTreeMap<String, String>,
        duration: Option<Duration>,
        ttl_count: Option<u64>,
    ) -> miette::Result<OneTimeCode>;
}

#[async_trait]
impl TokenIssuer for AuthorityNodeClient {
    async fn create_token(
        &self,
        ctx: &Context,
        attributes: BTreeMap<String, String>,
        duration: Option<Duration>,
        ttl_count: Option<u64>,
    ) -> miette::Result<OneTimeCode> {
        let body = CreateToken::new()
            .with_attributes(attributes)
            .with_ttl(duration)
            .with_ttl_count(ttl_count);

        let req = Request::post("/").body(body);
        self.get_secure_client()
            .ask(ctx, DefaultAddress::ENROLLMENT_TOKEN_ISSUER, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
