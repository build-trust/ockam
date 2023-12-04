use crate::authenticator::one_time_code::OneTimeCode;
use crate::authenticator::EnrollmentToken;
use ockam::identity::TimestampInSeconds;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;

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
