use crate::cloud::enroll::auth0::{AuthenticateOidcToken, OidcToken};
use crate::cloud::secure_client::SecureClient;
use crate::DefaultAddress;
use ockam_core::api::{Reply, Request};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_identity::{Credential, OneTimeCode};
use ockam_node::Context;

const TARGET: &str = "ockam_api::cloud::enroll";

#[async_trait]
pub trait Enrollment {
    async fn enroll_with_oidc_token(&self, ctx: &Context, token: OidcToken) -> Result<Reply<()>>;
    async fn enroll_with_oidc_token_okta(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> Result<Reply<()>>;
    async fn present_token(&self, ctx: &Context, token: &OneTimeCode) -> Result<Reply<()>>;
    async fn issue_credential(&self, ctx: &Context) -> Result<Reply<Credential>>;
}

#[async_trait]
impl Enrollment for SecureClient {
    async fn enroll_with_oidc_token(&self, ctx: &Context, token: OidcToken) -> Result<Reply<()>> {
        let req = Request::post("v0/enroll").body(AuthenticateOidcToken::new(token));
        trace!(target: TARGET, "executing auth0 flow");
        self.tell(ctx, "auth0_authenticator", req).await
    }

    async fn enroll_with_oidc_token_okta(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> Result<Reply<()>> {
        let req = Request::post("v0/enroll").body(AuthenticateOidcToken::new(token));
        trace!(target: TARGET, "executing auth0 flow");
        self.tell(ctx, DefaultAddress::OKTA_IDENTITY_PROVIDER, req)
            .await
    }

    async fn present_token(&self, ctx: &Context, token: &OneTimeCode) -> Result<Reply<()>> {
        let req = Request::post("/").body(token);
        trace!(target: TARGET, "present a token");
        self.tell(ctx, DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR, req)
            .await
    }

    async fn issue_credential(&self, ctx: &Context) -> Result<Reply<Credential>> {
        let req = Request::post("/");
        trace!(target: TARGET, "getting a credential");
        self.ask(ctx, DefaultAddress::CREDENTIAL_ISSUER, req).await
    }
}
