use crate::authenticator::one_time_code::OneTimeCode;
use ockam::identity::{Identifier, TimestampInSeconds};
use ockam_core::compat::str::FromStr;
use ockam_core::{Error, Result};
use std::collections::BTreeMap;

#[derive(Clone, Eq, PartialEq)]
pub struct EnrollmentToken {
    /// Sensitive random value returned to the enroller, so that an end-user can later present it
    /// to us and become a member
    pub one_time_code: OneTimeCode,
    /// Random string that uniquely identifies an enrollment token.
    /// However, unlike the one_time_code, it's not sensitive so can be logged
    /// and used to track a lifecycle of a specific enrollment token.
    pub reference: Option<String>,
    /// Issuer [`Identifier`]
    pub issued_by: Identifier,
    /// Created timestamp
    pub created_at: TimestampInSeconds,
    /// Expires timestamp
    pub expires_at: TimestampInSeconds,
    /// Number of times a [`OneTimeCode`] can be used (1 by default)
    pub ttl_count: u64,
    /// Attributes that will be assigned to a member upon usage of that token
    pub attrs: BTreeMap<String, String>,
}

impl EnrollmentToken {
    pub fn reference(&self) -> String {
        self.reference.clone().unwrap_or("NONE".to_string())
    }
}

// Low-level representation of a table row
#[derive(sqlx::FromRow)]
pub(crate) struct EnrollmentTokenRow {
    one_time_code: String,
    reference: Option<String>,
    issued_by: String,
    created_at: i64,
    expires_at: i64,
    ttl_count: i64,
    attributes: Vec<u8>,
}

impl TryFrom<EnrollmentTokenRow> for EnrollmentToken {
    type Error = Error;

    fn try_from(value: EnrollmentTokenRow) -> Result<Self, Self::Error> {
        let member = EnrollmentToken {
            one_time_code: OneTimeCode::from_str(&value.one_time_code)?,
            reference: value.reference,
            issued_by: Identifier::from_str(&value.issued_by)?,
            created_at: TimestampInSeconds(value.created_at as u64),
            expires_at: TimestampInSeconds(value.expires_at as u64),
            ttl_count: value.ttl_count as u64,
            attrs: minicbor::decode(&value.attributes)?,
        };

        Ok(member)
    }
}
