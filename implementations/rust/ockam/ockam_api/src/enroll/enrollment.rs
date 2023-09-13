use crate::cloud::enroll::auth0::{AuthenticateOidcToken, OidcToken};
use crate::cloud::secure_client::SecureClient;
use ockam_core::api::{Reply, Request};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::Context;

const TARGET: &str = "ockam_api::cloud::enroll";

#[async_trait]
pub trait Enrollment {
    async fn enroll_with_oidc_token(&self, ctx: &Context, token: OidcToken) -> Result<Reply<()>>;
}

#[async_trait]
impl Enrollment for SecureClient {
    async fn enroll_with_oidc_token(&self, ctx: &Context, token: OidcToken) -> Result<Reply<()>> {
        let req = Request::post("v0/enroll").body(AuthenticateOidcToken::new(token));
        trace!(target: TARGET, "executing auth0 flow");
        self.tell(ctx, "auth0_authenticator", req).await
    }
}
