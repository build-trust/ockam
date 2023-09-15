use crate::cloud::enroll::auth0::{AuthenticateOidcToken, OidcToken};
use crate::cloud::HasSecureClient;
use crate::DefaultAddress;
use miette::IntoDiagnostic;
use ockam_core::api::{Reply, Request, Status};
use ockam_core::async_trait;
use ockam_identity::{Credential, OneTimeCode, SecureClient};
use ockam_node::Context;

const TARGET: &str = "ockam_api::cloud::enroll";

pub enum EnrollStatus {
    EnrolledSuccessfully,
    AlreadyEnrolled,
    UnexpectedStatus(String, Status),
    FailedNoStatus(String),
}

#[async_trait]
pub trait Enrollment {
    async fn enroll_with_oidc_token(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<EnrollStatus>;

    async fn enroll_with_oidc_token_okta(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<()>;

    async fn present_token(&self, ctx: &Context, token: &OneTimeCode) -> miette::Result<()>;

    async fn issue_credential(&self, ctx: &Context) -> miette::Result<Credential>;
}

#[async_trait]
impl<T: HasSecureClient + Send + Sync> Enrollment for T {
    async fn enroll_with_oidc_token(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<EnrollStatus> {
        self.get_secure_client()
            .enroll_with_oidc_token(ctx, token)
            .await
    }

    async fn enroll_with_oidc_token_okta(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<()> {
        self.get_secure_client()
            .enroll_with_oidc_token_okta(ctx, token)
            .await
    }

    async fn present_token(&self, ctx: &Context, token: &OneTimeCode) -> miette::Result<()> {
        self.get_secure_client().present_token(ctx, token).await
    }

    async fn issue_credential(&self, ctx: &Context) -> miette::Result<Credential> {
        self.get_secure_client().issue_credential(ctx).await
    }
}

#[async_trait]
impl Enrollment for SecureClient {
    async fn enroll_with_oidc_token(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<EnrollStatus> {
        let req = Request::post("v0/enroll").body(AuthenticateOidcToken::new(token));
        trace!(target: TARGET, "executing auth0 flow");
        let reply = self
            .tell(ctx, "auth0_authenticator", req)
            .await
            .into_diagnostic()?;
        match reply {
            Reply::Successful(_) => Ok(EnrollStatus::EnrolledSuccessfully),
            Reply::Failed(_, Some(Status::BadRequest)) => Ok(EnrollStatus::AlreadyEnrolled),
            Reply::Failed(e, Some(s)) => Ok(EnrollStatus::UnexpectedStatus(e.to_string(), s)),
            Reply::Failed(e, _) => Ok(EnrollStatus::FailedNoStatus(e.to_string())),
        }
    }

    async fn enroll_with_oidc_token_okta(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<()> {
        let req = Request::post("v0/enroll").body(AuthenticateOidcToken::new(token));
        trace!(target: TARGET, "executing auth0 flow");
        self.tell(ctx, DefaultAddress::OKTA_IDENTITY_PROVIDER, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn present_token(&self, ctx: &Context, token: &OneTimeCode) -> miette::Result<()> {
        let req = Request::post("/").body(token);
        trace!(target: TARGET, "present a token");
        self.tell(ctx, DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn issue_credential(&self, ctx: &Context) -> miette::Result<Credential> {
        let req = Request::post("/");
        trace!(target: TARGET, "getting a credential");
        self.ask(ctx, DefaultAddress::CREDENTIAL_ISSUER, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
