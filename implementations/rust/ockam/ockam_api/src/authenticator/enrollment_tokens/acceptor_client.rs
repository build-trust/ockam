use miette::IntoDiagnostic;

use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;

use crate::authenticator::one_time_code::OneTimeCode;
use crate::cloud::{AuthorityNodeClient, HasSecureClient};
use crate::nodes::service::default_address::DefaultAddress;

#[async_trait]
pub trait TokenAcceptor {
    async fn present_token(&self, ctx: &Context, token: OneTimeCode) -> miette::Result<()>;
}

#[async_trait]
impl TokenAcceptor for AuthorityNodeClient {
    async fn present_token(&self, ctx: &Context, token: OneTimeCode) -> miette::Result<()> {
        let req = Request::post("/").body(token);
        self.get_secure_client()
            .tell(ctx, DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
