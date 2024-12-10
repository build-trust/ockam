use crate::authenticator::one_time_code::OneTimeCode;
use crate::authenticator::EnrollmentToken;
use ockam::identity::TimestampInSeconds;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::retry;

/// This repository stores enrollment tokens on the Authority node
#[async_trait]
pub trait AuthorityEnrollmentTokenRepository: Send + Sync + 'static {
    /// Use previously issued token
    async fn use_token(
        &self,
        one_time_code: OneTimeCode,
        now: TimestampInSeconds,
    ) -> Result<Option<EnrollmentToken>>;

    /// Store a newly issued enrolment token
    async fn store_new_token(&self, token: EnrollmentToken) -> Result<()>;
}

#[async_trait]
impl<T: AuthorityEnrollmentTokenRepository> AuthorityEnrollmentTokenRepository for AutoRetry<T> {
    async fn use_token(
        &self,
        one_time_code: OneTimeCode,
        now: TimestampInSeconds,
    ) -> Result<Option<EnrollmentToken>> {
        retry!(self.wrapped.use_token(one_time_code, now))
    }

    async fn store_new_token(&self, token: EnrollmentToken) -> Result<()> {
        retry!(self.wrapped.store_new_token(token.clone()))
    }
}
