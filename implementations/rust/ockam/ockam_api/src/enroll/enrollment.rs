use crate::authenticator::one_time_code::OneTimeCode;
use crate::nodes::service::default_address::DefaultAddress;
use crate::orchestrator::enroll::auth0::{AuthenticateOidcToken, OidcToken};
use crate::orchestrator::HasSecureClient;
use miette::IntoDiagnostic;
use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::SecureClient;
use ockam_core::api::{Reply, Request, Status};
use ockam_core::async_trait;
use ockam_node::Context;
use std::collections::HashMap;
use tracing::Level;

const TARGET: &str = "ockam_api::cloud::enroll";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnrollStatus {
    EnrolledSuccessfully,
    AlreadyEnrolled,
    UnexpectedStatus(String, Status),
    FailedNoStatus(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AiEnrollStatus {
    EnrolledSuccessfully(String),
    AlreadyEnrolled,
    UnexpectedStatus(String, Status),
    FailedNoStatus(String),
}

impl From<AiEnrollStatus> for EnrollStatus {
    fn from(val: AiEnrollStatus) -> Self {
        match val {
            AiEnrollStatus::EnrolledSuccessfully(_) => EnrollStatus::EnrolledSuccessfully,
            AiEnrollStatus::AlreadyEnrolled => EnrollStatus::AlreadyEnrolled,
            AiEnrollStatus::UnexpectedStatus(e, s) => EnrollStatus::UnexpectedStatus(e, s),
            AiEnrollStatus::FailedNoStatus(e) => EnrollStatus::FailedNoStatus(e),
        }
    }
}

#[async_trait]
pub trait Enrollment {
    async fn enroll_with_oidc_token(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<EnrollStatus>;

    async fn enroll_ai_with_oidc_token(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<AiEnrollStatus>;

    async fn enroll_with_oidc_token_okta(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<()>;

    async fn present_token(
        &self,
        ctx: &Context,
        token: &OneTimeCode,
    ) -> miette::Result<EnrollStatus>;

    async fn issue_credential(&self, ctx: &Context) -> miette::Result<CredentialAndPurposeKey>;

    async fn get_subject_attributes(
        &self,
        ctx: &Context,
    ) -> miette::Result<HashMap<Vec<u8>, Vec<u8>>>;
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

    async fn enroll_ai_with_oidc_token(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<AiEnrollStatus> {
        self.get_secure_client()
            .enroll_ai_with_oidc_token(ctx, token)
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

    async fn present_token(
        &self,
        ctx: &Context,
        token: &OneTimeCode,
    ) -> miette::Result<EnrollStatus> {
        self.get_secure_client().present_token(ctx, token).await
    }

    async fn issue_credential(&self, ctx: &Context) -> miette::Result<CredentialAndPurposeKey> {
        self.get_secure_client().issue_credential(ctx).await
    }

    async fn get_subject_attributes(
        &self,
        ctx: &Context,
    ) -> miette::Result<HashMap<Vec<u8>, Vec<u8>>> {
        self.get_secure_client().get_subject_attributes(ctx).await
    }
}

// FiXME: this has duplicate with AuthorityNodeClient
#[async_trait]
impl Enrollment for SecureClient {
    #[instrument(skip_all, level = Level::TRACE)]
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
            Reply::Failed(e, Some(s)) => {
                error!("enrolling with a token returned an error: {e:?}");
                Ok(EnrollStatus::UnexpectedStatus(e.to_string(), s))
            }
            Reply::Failed(e, _) => {
                error!("enrolling with a token returned an error: {e:?}");
                Ok(EnrollStatus::FailedNoStatus(e.to_string()))
            }
        }
    }

    #[instrument(skip_all, level = Level::TRACE)]
    async fn enroll_ai_with_oidc_token(
        &self,
        ctx: &Context,
        token: OidcToken,
    ) -> miette::Result<AiEnrollStatus> {
        let req = Request::post("v0/ai/enroll").body(AuthenticateOidcToken::new(token));
        trace!(target: TARGET, "executing ai auth0 flow");
        let reply = self
            .ask(ctx, "auth0_authenticator", req)
            .await
            .into_diagnostic()?;
        match reply {
            Reply::Successful(cluster) => Ok(AiEnrollStatus::EnrolledSuccessfully(cluster)),
            Reply::Failed(e, Some(s)) => {
                error!("enrolling with a token returned an error: {e:?}");
                Ok(AiEnrollStatus::UnexpectedStatus(e.to_string(), s))
            }
            Reply::Failed(e, _) => {
                error!("enrolling with a token returned an error: {e:?}");
                Ok(AiEnrollStatus::FailedNoStatus(e.to_string()))
            }
        }
    }

    #[instrument(skip_all, level = Level::TRACE)]
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

    #[instrument(skip_all, level = Level::TRACE)]
    async fn present_token(
        &self,
        ctx: &Context,
        token: &OneTimeCode,
    ) -> miette::Result<EnrollStatus> {
        let req = Request::post("/").body(*token);
        trace!(target: TARGET, "present a token");
        match self
            .tell(ctx, DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR, req)
            .await
            .into_diagnostic()?
        {
            Reply::Successful(_) => Ok(EnrollStatus::EnrolledSuccessfully),
            Reply::Failed(e, s) => match (e.message(), s) {
                // TODO: the `authenticator` should return proper error codes
                (Some(error), Some(Status::Forbidden)) => {
                    if error.to_lowercase().contains("already a member") {
                        Ok(EnrollStatus::AlreadyEnrolled)
                    } else {
                        Err(miette::miette!(e))
                    }
                }
                _ => Err(miette::miette!(e)),
            },
        }
    }

    #[instrument(skip_all, level = Level::TRACE)]
    async fn issue_credential(&self, ctx: &Context) -> miette::Result<CredentialAndPurposeKey> {
        let req = Request::post("/");
        trace!(target: TARGET, "getting a credential");
        self.ask(ctx, DefaultAddress::CREDENTIAL_ISSUER, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    #[instrument(skip_all, level = Level::TRACE)]
    async fn get_subject_attributes(
        &self,
        ctx: &Context,
    ) -> miette::Result<HashMap<Vec<u8>, Vec<u8>>> {
        let credential = self.issue_credential(ctx).await?;
        let attributes: HashMap<_, _> = credential
            .credential
            .get_credential_data()?
            .subject_attributes
            .map
            .into_iter()
            .map(|(k, v)| (Vec::<u8>::from(k), Vec::<u8>::from(v)))
            .collect();
        Ok(attributes)
    }
}
