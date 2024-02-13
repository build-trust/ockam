use either::Either;
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::collections::BTreeMap;

use ockam::identity::utils::now;
use ockam::identity::Identifier;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::time::Duration;
use ockam_core::Result;

use crate::authenticator::common::EnrollerAccessControlChecks;
use crate::authenticator::one_time_code::OneTimeCode;
use crate::authenticator::{
    AuthorityEnrollmentTokenRepository, AuthorityMembersRepository, EnrollmentToken,
};

pub(super) const MAX_TOKEN_DURATION: Duration = Duration::from_secs(600);

pub struct EnrollmentTokenIssuerError(pub String);

pub type EnrollmentTokenIssuerResult<T> = Either<T, EnrollmentTokenIssuerError>;

pub struct EnrollmentTokenIssuer {
    pub(super) tokens: Arc<dyn AuthorityEnrollmentTokenRepository>,
    pub(super) members: Arc<dyn AuthorityMembersRepository>,
}

impl EnrollmentTokenIssuer {
    pub fn new(
        tokens: Arc<dyn AuthorityEnrollmentTokenRepository>,
        members: Arc<dyn AuthorityMembersRepository>,
    ) -> Self {
        Self { tokens, members }
    }

    #[instrument(skip_all, fields(enroller = %enroller, token_duration = token_duration.map_or("n/a".to_string(), |d| d.as_secs().to_string()), ttl_count = ttl_count.map_or("n/a".to_string(), |t| t.to_string())))]
    pub async fn issue_token(
        &self,
        enroller: &Identifier,
        attrs: BTreeMap<String, String>,
        token_duration: Option<Duration>,
        ttl_count: Option<u64>,
    ) -> Result<EnrollmentTokenIssuerResult<OneTimeCode>> {
        let check =
            EnrollerAccessControlChecks::check_identifier(self.members.clone(), enroller).await?;

        if !check.is_enroller {
            warn!(
                "Non-enroller {} is trying to issue an enrollment token",
                enroller
            );
            return Ok(Either::Right(EnrollmentTokenIssuerError(
                "Non-enroller is trying to issue an enrollment token".to_string(),
            )));
        }

        // Check if we're trying to create an enroller
        if EnrollerAccessControlChecks::check_str_attributes_is_enroller(&attrs) {
            // Only pre-trusted identities will be able to add enrollers
            if !check.is_pre_trusted {
                warn!("Not pre trusted enroller {} is trying to issue an enrollment token for an enroller", enroller);
                return Ok(Either::Right(EnrollmentTokenIssuerError(
                    "Not pre trusted enroller is trying to issue an enrollment token for an enroller".to_string(),
                )));
            }
        }

        let one_time_code = OneTimeCode::new();
        let reference: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
        let max_token_duration = token_duration.unwrap_or(MAX_TOKEN_DURATION);
        let ttl_count = ttl_count.unwrap_or(1);
        let now = now()?;
        let expires_at = now + max_token_duration.as_secs();
        let tkn = EnrollmentToken {
            one_time_code: one_time_code.clone(),
            reference: Some(reference.clone()),
            issued_by: enroller.clone(),
            created_at: now,
            expires_at,
            ttl_count,
            attrs,
        };
        self.tokens.store_new_token(tkn).await?;

        info!(
            "Successfully issued an enrollment token. TTL count: {}, expires_at: {}, reference: {}",
            ttl_count, expires_at.0, reference
        );

        Ok(Either::Left(one_time_code))
    }
}
