use either::Either;
use ockam::identity::utils::now;
use ockam::identity::Identifier;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

use crate::authenticator::common::EnrollerAccessControlChecks;
use crate::authenticator::one_time_code::OneTimeCode;
use crate::authenticator::{
    AuthorityEnrollmentTokenRepository, AuthorityMember, AuthorityMembersRepository,
};

pub struct EnrollmentTokenAcceptorError(pub String);

pub type EnrollmentTokenAcceptorResult<T> = Either<T, EnrollmentTokenAcceptorError>;

pub struct EnrollmentTokenAcceptor {
    pub(super) tokens: Arc<dyn AuthorityEnrollmentTokenRepository>,
    pub(super) members: Arc<dyn AuthorityMembersRepository>,
}

impl EnrollmentTokenAcceptor {
    pub fn new(
        tokens: Arc<dyn AuthorityEnrollmentTokenRepository>,
        members: Arc<dyn AuthorityMembersRepository>,
    ) -> Self {
        Self { tokens, members }
    }

    #[instrument(skip_all, fields(from = %from))]
    pub async fn accept_token(
        &mut self,
        otc: OneTimeCode,
        from: &Identifier,
    ) -> Result<EnrollmentTokenAcceptorResult<()>> {
        let check =
            EnrollerAccessControlChecks::check_identifier(self.members.clone(), from).await?;

        // Not allow updating existing members
        if check.is_member {
            warn!("{} is already a member", from);
            return Ok(Either::Right(EnrollmentTokenAcceptorError(
                "Already a member".to_string(),
            )));
        }

        let token = match self.tokens.use_token(otc, now()?).await {
            Ok(Some(token)) => token,
            Ok(None) => {
                warn!("Unknown enrollment token received from {}", from);
                return Ok(Either::Right(EnrollmentTokenAcceptorError(
                    "Unknown enrollment token".to_string(),
                )));
            }
            Err(err) => {
                warn!(
                    "Error using an enrollment token received from {}. Error: {}",
                    from, err
                );
                return Ok(Either::Right(EnrollmentTokenAcceptorError(
                    "Error using the enrollment token".to_string(),
                )));
            }
        };

        let reference = token.reference();
        let attrs = token
            .attrs
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .collect();

        let member = AuthorityMember::new(from.clone(), attrs, token.issued_by, now()?, false);

        if let Err(err) = self.members.add_member(member).await {
            warn!(
                "Error adding member {} using enrollment token: {}",
                from, err
            );
            return Ok(Either::Right(EnrollmentTokenAcceptorError(
                "Error adding member using enrollment token".to_string(),
            )));
        }

        info!(
            "Successfully accepted an enrollment token from {}. Reference: {}",
            from, reference
        );

        Ok(Either::Left(()))
    }
}
