use crate::authenticator::one_time_code::OneTimeCode;
use ockam::identity::{Identifier, TimestampInSeconds};
use ockam_core::compat::str::FromStr;
use ockam_core::{Error, Result};
use std::collections::BTreeMap;

#[derive(Clone, Eq, PartialEq)]
pub struct EnrollmentToken {
    pub one_time_code: OneTimeCode,
    pub issued_by: Identifier,
    pub created_at: TimestampInSeconds,
    pub expires_at: TimestampInSeconds,
    pub ttl_count: u64,
    pub attrs: BTreeMap<String, String>,
}

// Low-level representation of a table row
#[derive(sqlx::FromRow)]
pub(crate) struct EnrollmentTokenRow {
    one_time_code: String,
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
            issued_by: Identifier::from_str(&value.issued_by)?,
            created_at: TimestampInSeconds(value.created_at as u64),
            expires_at: TimestampInSeconds(value.expires_at as u64),
            ttl_count: value.ttl_count as u64,
            attrs: minicbor::decode(&value.attributes)?,
        };

        Ok(member)
    }
}
